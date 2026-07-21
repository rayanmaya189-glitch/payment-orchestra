use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use tracing::info;

use crate::application::commands::*;
use crate::domain::aggregates::Dispute;
use crate::domain::entities::{Evidence, EvidenceType};
use crate::domain::rules::{DisputeRepository, EvidenceRepository};
use crate::domain::value_objects::DisputeDecision;
use platform_error::{PlatformError, ValidationError};
use platform_logging::log_security_event;
use platform_logging::{SecurityEventType, SecurityOutcome};
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::{CurrencyCode, Money};

pub struct DisputeServiceImpl {
    repo: Box<dyn DisputeRepository>,
    evidence_repo: Box<dyn EvidenceRepository>,
    db: DatabaseConnection,
}

impl DisputeServiceImpl {
    pub fn new(
        repo: Box<dyn DisputeRepository>,
        evidence_repo: Box<dyn EvidenceRepository>,
        db: DatabaseConnection,
    ) -> Self {
        Self {
            repo,
            evidence_repo,
            db,
        }
    }

    fn check_abac(
        principal_id: Uuid,
        role: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), PlatformError> {
        evaluate_policy(&AbacContext {
            principal_id,
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        })
    }
}

#[async_trait]
pub trait DisputeService: Send + Sync {
    async fn open(&self, cmd: OpenDisputeCommand) -> Result<Uuid, PlatformError>;
    async fn submit_evidence(&self, cmd: SubmitEvidenceCommand) -> Result<(), PlatformError>;
    async fn resolve(&self, cmd: ResolveDisputeCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<Dispute, PlatformError>;
    async fn list(&self, operator_id: Uuid) -> Result<Vec<Dispute>, PlatformError>;
}

#[async_trait]
impl DisputeService for DisputeServiceImpl {
    async fn open(&self, cmd: OpenDisputeCommand) -> Result<Uuid, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "create", "dispute")?;

        let amount = Money {
            amount_minor_units: cmd.amount_minor_units,
            currency: CurrencyCode::new(&cmd.currency)
                .map_err(|_| PlatformError::Validation(ValidationError::InvalidCurrencyCode))?,
        };

        let d = Dispute::new(
            cmd.payment_intent_id,
            cmd.operator_id,
            cmd.reason,
            amount,
            cmd.acquirer_reference,
            cmd.connector_id,
        )?;

        let dispute_id = d.dispute_id;
        self.repo.save(&d).await?;

        info!(
            dispute_id = %dispute_id,
            payment_intent_id = %d.payment_intent_id,
            operator_id = %cmd.operator_id,
            principal_id = %cmd.principal_id,
            "Dispute opened"
        );

        Ok(dispute_id)
    }

    async fn submit_evidence(&self, cmd: SubmitEvidenceCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "dispute")?;

        let mut d = self
            .repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "dispute".into(),
                id: cmd.dispute_id,
            })?;

        // Ownership check: only the operator that opened the dispute can submit evidence
        if d.operator_id != cmd.operator_id {
            log_security_event(
                "dispute-service",
                SecurityEventType::PermissionDenied,
                SecurityOutcome::Blocked,
                Some(cmd.principal_id),
                None,
                None,
                Some(cmd.dispute_id),
                Some(serde_json::json!({
                    "action": "submit_evidence",
                    "dispute_id": cmd.dispute_id,
                    "attempted_by_operator": cmd.principal_id,
                    "dispute_owner": d.operator_id,
                })),
            );
            return Err(PlatformError::AuthorizationDenied(
                "Only the dispute owner operator can submit evidence".into(),
            ));
        }

        d.submit_evidence(cmd.evidence.clone())?;

        // Persist evidence to the evidence repository
        let evidence = Evidence::new(
            cmd.dispute_id,
            EvidenceType::Other,
            serde_json::to_string(&cmd.evidence).unwrap_or_default(),
            None,
            cmd.principal_id,
        );
        self.evidence_repo.add_evidence(cmd.dispute_id, &evidence).await?;
        self.repo.save(&d).await?;

        info!(
            dispute_id = %cmd.dispute_id,
            principal_id = %cmd.principal_id,
            "Evidence submitted"
        );

        Ok(())
    }

    async fn resolve(&self, cmd: ResolveDisputeCommand) -> Result<(), PlatformError> {
        // Compliance officer role check: only compliance_officer and platform_admin can resolve
        if !matches!(cmd.role.as_str(), "compliance_officer" | "platform_admin") {
            log_security_event(
                "dispute-service",
                SecurityEventType::PermissionDenied,
                SecurityOutcome::Blocked,
                Some(cmd.principal_id),
                None,
                None,
                Some(cmd.dispute_id),
                Some(serde_json::json!({
                    "action": "resolve_dispute",
                    "dispute_id": cmd.dispute_id,
                    "role": cmd.role,
                })),
            );
            return Err(PlatformError::AuthorizationDenied(
                "Only compliance officers and platform admins can resolve disputes".into(),
            ));
        }

        let decision = DisputeDecision::from_str(&cmd.decision).ok_or_else(|| {
            PlatformError::Validation(ValidationError::MissingField(format!(
                "invalid decision: {}",
                cmd.decision
            )))
        })?;

        let mut d = self
            .repo
            .find_by_id(cmd.dispute_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "dispute".into(),
                id: cmd.dispute_id,
            })?;

        d.resolve(decision, &cmd.reason)?;
        self.repo.save(&d).await?;

        info!(
            dispute_id = %cmd.dispute_id,
            decision = %cmd.decision,
            principal_id = %cmd.principal_id,
            "Dispute resolved"
        );

        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Dispute, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| {
            PlatformError::NotFound {
                resource: "dispute".into(),
                id,
            }
        })
    }

    async fn list(&self, operator_id: Uuid) -> Result<Vec<Dispute>, PlatformError> {
        self.repo.list_by_operator(operator_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use crate::domain::value_objects::DisputeStatus;

    // ── In-memory test doubles ──────────────────────────────────────────

    struct InMemoryDisputeRepo {
        store: Mutex<HashMap<Uuid, Dispute>>,
    }

    impl InMemoryDisputeRepo {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl DisputeRepository for InMemoryDisputeRepo {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<Dispute>, PlatformError> {
            Ok(self.store.lock().unwrap().get(&id).cloned())
        }
        async fn save(&self, dispute: &Dispute) -> Result<(), PlatformError> {
            self.store
                .lock()
                .unwrap()
                .insert(dispute.dispute_id, dispute.clone());
            Ok(())
        }
        async fn list_by_operator(&self, operator_id: Uuid) -> Result<Vec<Dispute>, PlatformError> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .values()
                .filter(|d| d.operator_id == operator_id)
                .cloned()
                .collect())
        }
    }

    struct InMemoryEvidenceRepo;

    #[async_trait]
    impl EvidenceRepository for InMemoryEvidenceRepo {
        async fn add_evidence(&self, _dispute_id: Uuid, _evidence: &Evidence) -> Result<(), PlatformError> {
            Ok(())
        }
        async fn get_evidence(&self, _dispute_id: Uuid) -> Result<Vec<Evidence>, PlatformError> {
            Ok(vec![])
        }
    }

    fn make_service() -> DisputeServiceImpl {
        let db = sea_orm::DatabaseConnection::default();
        DisputeServiceImpl::new(
            Box::new(InMemoryDisputeRepo::new()),
            Box::new(InMemoryEvidenceRepo),
            db,
        )
    }

    fn aed(amount: i64) -> Money {
        Money {
            amount_minor_units: amount,
            currency: CurrencyCode::new("AED").unwrap(),
        }
    }

    // ── Tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_open_dispute_success() {
        let svc = make_service();
        let id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: Uuid::now_v7(),
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();
        let d = svc.get(id).await.unwrap();
        assert_eq!(d.status, DisputeStatus::Opened);
    }

    #[tokio::test]
    async fn test_open_dispute_rejects_invalid_currency() {
        let svc = make_service();
        let result = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: Uuid::now_v7(),
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "ae".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_open_dispute_rejects_read_only_role() {
        let svc = make_service();
        let result = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: Uuid::now_v7(),
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "read_only".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_submit_evidence_success() {
        let svc = make_service();
        let operator_id = Uuid::now_v7();
        let dispute_id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id,
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();

        svc.submit_evidence(SubmitEvidenceCommand {
            dispute_id,
            evidence: serde_json::json!({"docs": ["receipt.pdf"]}),
            principal_id: Uuid::now_v7(),
            role: "operator_admin".into(),
            operator_id,
        })
        .await
        .unwrap();

        let d = svc.get(dispute_id).await.unwrap();
        assert_eq!(d.status, DisputeStatus::EvidenceSubmitted);
    }

    #[tokio::test]
    async fn test_submit_evidence_rejects_non_owner() {
        let svc = make_service();
        let owner = Uuid::now_v7();
        let dispute_id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: owner,
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();

        let result = svc
            .submit_evidence(SubmitEvidenceCommand {
                dispute_id,
                evidence: serde_json::json!({}),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
                operator_id: Uuid::now_v7(), // different operator
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_success() {
        let svc = make_service();
        let operator_id = Uuid::now_v7();
        let dispute_id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id,
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();

        svc.submit_evidence(SubmitEvidenceCommand {
            dispute_id,
            evidence: serde_json::json!({"docs": []}),
            principal_id: Uuid::now_v7(),
            role: "operator_admin".into(),
            operator_id,
        })
        .await
        .unwrap();

        svc.resolve(ResolveDisputeCommand {
            dispute_id,
            decision: "won".into(),
            reason: "Evidence sufficient".into(),
            principal_id: Uuid::now_v7(),
            role: "compliance_officer".into(),
        })
        .await
        .unwrap();

        let d = svc.get(dispute_id).await.unwrap();
        assert_eq!(d.status, DisputeStatus::Resolved);
        assert_eq!(d.decision, Some(DisputeDecision::Won));
    }

    #[tokio::test]
    async fn test_resolve_rejects_non_compliance_role() {
        let svc = make_service();
        let dispute_id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: Uuid::now_v7(),
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();

        let result = svc
            .resolve(ResolveDisputeCommand {
                dispute_id,
                decision: "won".into(),
                reason: "Evidence sufficient".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_rejects_invalid_decision() {
        let svc = make_service();
        let dispute_id = svc
            .open(OpenDisputeCommand {
                payment_intent_id: Uuid::now_v7(),
                operator_id: Uuid::now_v7(),
                reason: "fraud".into(),
                amount_minor_units: 5000,
                currency: "AED".into(),
                acquirer_reference: "acq_1".into(),
                connector_id: "ni".into(),
                principal_id: Uuid::now_v7(),
                role: "operator_admin".into(),
            })
            .await
            .unwrap();

        let result = svc
            .resolve(ResolveDisputeCommand {
                dispute_id,
                decision: "maybe".into(),
                reason: "Unknown".into(),
                principal_id: Uuid::now_v7(),
                role: "compliance_officer".into(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_by_operator() {
        let svc = make_service();
        let op1 = Uuid::now_v7();
        let op2 = Uuid::now_v7();

        svc.open(OpenDisputeCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: op1,
            reason: "fraud".into(),
            amount_minor_units: 1000,
            currency: "AED".into(),
            acquirer_reference: "acq_a".into(),
            connector_id: "ni".into(),
            principal_id: Uuid::now_v7(),
            role: "operator_admin".into(),
        })
        .await
        .unwrap();

        svc.open(OpenDisputeCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: op2,
            reason: "duplicate".into(),
            amount_minor_units: 2000,
            currency: "AED".into(),
            acquirer_reference: "acq_b".into(),
            connector_id: "ni".into(),
            principal_id: Uuid::now_v7(),
            role: "operator_admin".into(),
        })
        .await
        .unwrap();

        let list = svc.list(op1).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].operator_id, op1);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let svc = make_service();
        let result = svc.get(Uuid::now_v7()).await;
        assert!(matches!(result, Err(PlatformError::NotFound { .. })));
    }
}

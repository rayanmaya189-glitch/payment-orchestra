use async_trait::async_trait;
use uuid::Uuid;

use crate::application::commands::AssessPaymentRiskCommand;
use crate::domain::aggregates::RiskAssessment;
use crate::domain::rules::{default_rules, RiskAssessmentRepository};
use crate::domain::value_objects::{PaymentContext, RiskThresholds};
use platform_error::PlatformError;

/// Core risk-service operations.
#[async_trait]
pub trait RiskService: Send + Sync {
    /// Assess the risk of a payment and persist the result.
    async fn assess(&self, cmd: AssessPaymentRiskCommand) -> Result<RiskAssessment, PlatformError>;

    /// Retrieve a previously persisted assessment by its ID.
    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessment, PlatformError>;
}

/// Production implementation backed by a repository and configurable thresholds.
pub struct RiskServiceImpl {
    repo: Box<dyn RiskAssessmentRepository>,
    thresholds: RiskThresholds,
}

impl RiskServiceImpl {
    pub fn new(repo: Box<dyn RiskAssessmentRepository>, thresholds: RiskThresholds) -> Self {
        Self { repo, thresholds }
    }
}

#[async_trait]
impl RiskService for RiskServiceImpl {
    async fn assess(&self, cmd: AssessPaymentRiskCommand) -> Result<RiskAssessment, PlatformError> {
        // --- ABAC: authorization check ---
        if !cmd.is_authorized() {
            tracing::warn!(
                principal_role = %cmd.principal_role,
                operator_id = %cmd.operator_id,
                "Risk assessment denied: ABAC check failed"
            );
            return Err(PlatformError::AuthorizationDenied(format!(
                "Principal with role '{}' is not authorized to assess risk for operator {}",
                cmd.principal_role, cmd.operator_id
            )));
        }

        tracing::info!(
            payment_intent_id = %cmd.payment_intent_id,
            operator_id = %cmd.operator_id,
            "Starting risk assessment"
        );

        // --- Build payment context from command ---
        let ctx = PaymentContext {
            payment_intent_id: cmd.payment_intent_id,
            operator_id: cmd.operator_id,
            amount_minor_units: cmd.amount_minor_units,
            currency: cmd.currency,
            ip_address: cmd.ip_address.clone(),
            user_agent: cmd.user_agent.clone(),
            country_code: cmd.country_code,
            merchant_country: cmd.merchant_country,
            is_whitelisted: cmd.is_whitelisted,
            is_blacklisted: cmd.is_blacklisted,
            recent_tx_count_from_ip: cmd.recent_tx_count_from_ip,
            recent_tx_count_from_card: cmd.recent_tx_count_from_card,
        };

        // --- Evaluate rules ---
        let mut assessment = RiskAssessment::new(cmd.payment_intent_id, cmd.operator_id);
        assessment.ip_address = cmd.ip_address;
        assessment.user_agent = cmd.user_agent;

        let rules = default_rules();
        assessment.evaluate(&ctx, &rules, &self.thresholds);

        tracing::info!(
            assessment_id = %assessment.assessment_id,
            score = assessment.score,
            decision = assessment.decision.as_str(),
            "Risk assessment complete"
        );

        // --- Persist ---
        self.repo.save(&assessment).await?;

        Ok(assessment)
    }

    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessment, PlatformError> {
        tracing::debug!(assessment_id = %id, "Fetching risk assessment");
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "risk_assessment".into(),
                id,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rules::RiskAssessmentRepository;
    use crate::domain::value_objects::RiskDecision;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // --- In-memory repository for tests ---

    struct InMemoryRepo {
        store: Mutex<HashMap<Uuid, RiskAssessment>>,
    }

    impl InMemoryRepo {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl RiskAssessmentRepository for InMemoryRepo {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, PlatformError> {
            Ok(self.store.lock().unwrap().get(&id).cloned())
        }

        async fn save(&self, assessment: &RiskAssessment) -> Result<(), PlatformError> {
            self.store
                .lock()
                .unwrap()
                .insert(assessment.assessment_id, assessment.clone());
            Ok(())
        }
    }

    fn base_cmd() -> AssessPaymentRiskCommand {
        AssessPaymentRiskCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 100_00,
            currency: "AED".into(),
            ip_address: Some("203.0.113.1".into()),
            user_agent: Some("Mozilla/5.0".into()),
            country_code: Some("AE".into()),
            merchant_country: Some("AE".into()),
            is_whitelisted: false,
            is_blacklisted: false,
            recent_tx_count_from_ip: 1,
            recent_tx_count_from_card: 1,
            principal_role: "operator_admin".into(),
            principal_operator_id: Uuid::nil(), // will be set below
        }
    }

    #[tokio::test]
    async fn assess_low_risk_returns_allow() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.principal_operator_id = cmd.operator_id; // same operator → authorized

        let result = service.assess(cmd).await;
        assert!(result.is_ok());
        let assessment = result.unwrap();
        assert_eq!(assessment.decision, RiskDecision::Allow);
        assert!(assessment.score <= 0.4);
    }

    #[tokio::test]
    async fn assess_high_amount_returns_higher_score() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.amount_minor_units = 800_00; // above threshold
        cmd.principal_operator_id = cmd.operator_id;

        let assessment = service.assess(cmd).await.unwrap();
        assert!(assessment.score > 0.0);
        assert!(assessment.factors.iter().any(|f| f.rule_name == "high_amount"));
    }

    #[tokio::test]
    async fn assess_unauthorized_principal() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.principal_role = "read_only".into();

        let result = service.assess(cmd).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            PlatformError::AuthorizationDenied(_) => {}
            other => panic!("Expected AuthorizationDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn assess_persists_and_retrievable() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.principal_operator_id = cmd.operator_id;

        let assessment = service.assess(cmd).await.unwrap();
        let fetched = service.get_assessment(assessment.assessment_id).await.unwrap();
        assert_eq!(fetched.assessment_id, assessment.assessment_id);
        assert_eq!(fetched.score, assessment.score);
        assert_eq!(fetched.decision, assessment.decision);
    }

    #[tokio::test]
    async fn get_nonexistent_assessment() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let result = service.get_assessment(Uuid::now_v7()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            PlatformError::NotFound { .. } => {}
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn assess_blacklisted_entity() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.is_blacklisted = true;
        cmd.principal_operator_id = cmd.operator_id;

        let assessment = service.assess(cmd).await.unwrap();
        assert_eq!(assessment.decision, RiskDecision::Decline);
        assert!((assessment.score - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn assess_whitelisted_entity() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        cmd.is_whitelisted = true;
        cmd.amount_minor_units = 10_000_00; // huge amount
        cmd.principal_operator_id = cmd.operator_id;

        let assessment = service.assess(cmd).await.unwrap();
        assert_eq!(assessment.decision, RiskDecision::Allow);
    }

    #[tokio::test]
    async fn assess_operator_admin_wrong_operator_denied() {
        let repo = InMemoryRepo::new();
        let service = RiskServiceImpl::new(Box::new(repo), RiskThresholds::default());

        let mut cmd = base_cmd();
        // principal_operator_id differs from cmd.operator_id
        cmd.principal_operator_id = Uuid::now_v7();
        cmd.operator_id = Uuid::now_v7();

        let result = service.assess(cmd).await;
        assert!(result.is_err());
    }
}

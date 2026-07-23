//! Command handlers for BC-03 Merchant Compliance.

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::domain::{KybCase, AmlAlert, AmlMonitor, ComplianceError};
use crate::events::{ComplianceEvent, KybCaseSubmitted, KybCaseApproved, KybCaseRejected, AmlAlertCreated};
use crate::repository::ComplianceRepository;
use crate::commands::types::*;

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn submit_kyb_evidence(&self, cmd: SubmitKybEvidence) -> Result<SubmitKybEvidenceResult, ComplianceError>;
    async fn review_kyb_case(&self, cmd: ReviewKybCase) -> Result<ReviewKybCaseResult, ComplianceError>;
    async fn scan_transaction(&self, cmd: ScanTransaction) -> Result<ScanTransactionResult, ComplianceError>;
    async fn review_aml_alert(&self, cmd: ReviewAmlAlert) -> Result<ReviewAmlAlertResult, ComplianceError>;
}

// ─── Command Handler ────────────────────────────────────────────────────────

pub struct ComplianceCommandHandler<R: ComplianceRepository> {
    repository: R,
    aml_monitor: AmlMonitor,
    event_bus: Option<Arc<dyn EventBus>>,
}

impl<R: ComplianceRepository> ComplianceCommandHandler<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            aml_monitor: AmlMonitor::new(),
            event_bus: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_event_bus(mut self, event_bus: Arc<dyn EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    fn publish_event(&self, event: ComplianceEvent) {
        match Self::encode_event_proto(&event) {
            Ok(payload) => {
                publish_event_fire_and_forget(&self.event_bus, "compliance", event.event_type(), payload);
            }
            Err(e) => {
                tracing::error!(error = %e, event_type = %event.event_type(), "Failed to encode event as protobuf");
            }
        }
    }

    /// Encode a ComplianceEvent as protobuf bytes using the generated proto types.
    fn encode_event_proto(event: &ComplianceEvent) -> Result<Vec<u8>, String> {
        match event {
            ComplianceEvent::KybCaseSubmitted(e) => {
                let proto = platform_proto::compliance::KybCaseSubmittedEvent {
                    kyb_case_id: e.kyb_case_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    document_count: e.document_count as i32,
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            ComplianceEvent::KybCaseApproved(e) => {
                let proto = platform_proto::compliance::KybCaseApprovedEvent {
                    kyb_case_id: e.kyb_case_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            ComplianceEvent::KybCaseRejected(e) => {
                let proto = platform_proto::compliance::KybCaseRejectedEvent {
                    kyb_case_id: e.kyb_case_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            ComplianceEvent::AmlAlertCreated(e) => {
                let proto = platform_proto::compliance::AmlAlertCreatedEvent {
                    alert_id: e.alert_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    alert_type: e.alert_type.clone(),
                    severity: e.severity.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
        }
    }
}

#[async_trait::async_trait]
impl<R: ComplianceRepository + Send + Sync> CommandHandler for ComplianceCommandHandler<R> {
    async fn submit_kyb_evidence(&self, cmd: SubmitKybEvidence) -> Result<SubmitKybEvidenceResult, ComplianceError> {
        if cmd.document_ids.is_empty() {
            return Err(ComplianceError::KybNoDocuments);
        }

        let kase = KybCase::new(
            cmd.operator_id,
            cmd.submitted_by,
            cmd.document_ids,
        );

        self.repository.save_kyb_case(&kase).await?;

        self.publish_event(ComplianceEvent::KybCaseSubmitted(KybCaseSubmitted {
            kyb_case_id: kase.kyb_case_id,
            operator_id: kase.operator_id,
            document_count: kase.document_ids.len() as u32,
            occurred_at: Utc::now(),
        }));

        info!(
            kyb_case_id = %kase.kyb_case_id,
            operator_id = %kase.operator_id,
            "KYB evidence submitted"
        );

        Ok(SubmitKybEvidenceResult { kyb_case: kase })
    }

    async fn review_kyb_case(&self, cmd: ReviewKybCase) -> Result<ReviewKybCaseResult, ComplianceError> {
        let mut kase = self.repository.load_kyb_case(cmd.kyb_case_id).await?
            .ok_or(ComplianceError::KybCaseNotFound(cmd.kyb_case_id))?;

        if cmd.approved {
            kase.approve()?;
            self.publish_event(ComplianceEvent::KybCaseApproved(KybCaseApproved {
                kyb_case_id: kase.kyb_case_id,
                operator_id: kase.operator_id,
                occurred_at: Utc::now(),
            }));
            info!(kyb_case_id = %kase.kyb_case_id, "KYB case approved");
        } else {
            let reason = cmd.reason.unwrap_or_else(|| "No reason provided".into());
            kase.reject(reason.clone())?;
            self.publish_event(ComplianceEvent::KybCaseRejected(KybCaseRejected {
                kyb_case_id: kase.kyb_case_id,
                operator_id: kase.operator_id,
                reason,
                occurred_at: Utc::now(),
            }));
            info!(kyb_case_id = %kase.kyb_case_id, "KYB case rejected");
        }

        self.repository.save_kyb_case(&kase).await?;

        Ok(ReviewKybCaseResult { kyb_case: kase })
    }

    async fn scan_transaction(&self, cmd: ScanTransaction) -> Result<ScanTransactionResult, ComplianceError> {
        let recent_txns = self.repository.get_recent_transactions(cmd.operator_id, 60).await?;
        let recent_by_method = if let Some(ref method_id) = cmd.payment_method_id {
            self.repository.get_recent_by_method(method_id, 300).await?
        } else {
            vec![]
        };
        let avg_amount = self.repository.get_average_amount(cmd.operator_id, 30).await?;

        let alerts = self.aml_monitor.scan(
            cmd.transaction_id,
            cmd.operator_id,
            cmd.amount_minor_units,
            &recent_txns,
            &recent_by_method,
            avg_amount,
        );

        for alert in &alerts {
            self.repository.save_aml_alert(alert).await?;
            self.publish_event(ComplianceEvent::AmlAlertCreated(AmlAlertCreated {
                alert_id: alert.alert_id,
                operator_id: alert.operator_id,
                alert_type: alert.alert_type.as_str().to_string(),
                severity: alert.severity.as_str().to_string(),
                occurred_at: Utc::now(),
            }));
        }

        let blocked = alerts.iter().any(|a| a.severity.as_str() == "critical");

        if !alerts.is_empty() {
            info!(
                transaction_id = %cmd.transaction_id,
                alert_count = alerts.len(),
                blocked = blocked,
                "AML scan completed"
            );
        }

        Ok(ScanTransactionResult { alerts, blocked })
    }

    async fn review_aml_alert(&self, cmd: ReviewAmlAlert) -> Result<ReviewAmlAlertResult, ComplianceError> {
        let mut alert = self.repository.load_aml_alert(cmd.alert_id).await?
            .ok_or(ComplianceError::AmlAlertNotFound(cmd.alert_id))?;

        if alert.status != crate::domain::AlertStatus::Open {
            return Err(ComplianceError::AmlAlertAlreadyReviewed(cmd.alert_id));
        }

        match cmd.decision {
            AmlAlertDecision::Escalated => {
                alert.status = crate::domain::AlertStatus::Escalated;
            }
            AmlAlertDecision::ClosedFalsePositive => {
                alert.status = crate::domain::AlertStatus::Closed;
            }
        }

        alert.reviewed_by = Some(cmd.reviewer_id);
        alert.reviewed_at = Some(Utc::now());
        self.repository.save_aml_alert(&alert).await?;

        info!(
            alert_id = %alert.alert_id,
            status = %alert.status.as_str(),
            "AML alert reviewed"
        );

        Ok(ReviewAmlAlertResult { alert })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AlertStatus;
    use crate::repository::InMemoryComplianceRepository;

    async fn setup() -> ComplianceCommandHandler<InMemoryComplianceRepository> {
        let repo = InMemoryComplianceRepository::new();
        ComplianceCommandHandler::new(repo)
    }

    #[tokio::test]
    async fn test_submit_kyb_evidence_success() {
        let handler = setup().await;
        let result = handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: Uuid::now_v7(),
            document_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            submitted_by: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.kyb_case.status.as_str(), "submitted");
        assert_eq!(result.kyb_case.document_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_submit_kyb_no_documents_rejected() {
        let handler = setup().await;
        let result = handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: Uuid::now_v7(),
            document_ids: vec![],
            submitted_by: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_kyb_approve() {
        let handler = setup().await;
        let submitted = handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: Uuid::now_v7(),
            document_ids: vec![Uuid::now_v7()],
            submitted_by: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.review_kyb_case(ReviewKybCase {
            kyb_case_id: submitted.kyb_case.kyb_case_id,
            approved: true,
            reason: None,
            _reviewed_by: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.kyb_case.status.as_str(), "approved");
    }

    #[tokio::test]
    async fn test_kyb_reject() {
        let handler = setup().await;
        let submitted = handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: Uuid::now_v7(),
            document_ids: vec![Uuid::now_v7()],
            submitted_by: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.review_kyb_case(ReviewKybCase {
            kyb_case_id: submitted.kyb_case.kyb_case_id,
            approved: false,
            reason: Some("Invalid documents".into()),
            _reviewed_by: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.kyb_case.status.as_str(), "rejected");
        assert_eq!(result.kyb_case.rejection_reason.unwrap(), "Invalid documents");
    }

    #[tokio::test]
    async fn test_scan_transaction_no_alerts() {
        let handler = setup().await;
        let result = handler.scan_transaction(ScanTransaction {
            transaction_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 1000,
            payment_method_id: None,
        }).await.unwrap();

        assert!(result.alerts.is_empty());
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn test_review_aml_alert() {
        let handler = setup().await;
        let scan = handler.scan_transaction(ScanTransaction {
            transaction_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 600_000_00, // 600,000 AED (above absolute threshold)
            payment_method_id: None,
        }).await.unwrap();

        assert!(!scan.alerts.is_empty());

        let result = handler.review_aml_alert(ReviewAmlAlert {
            alert_id: scan.alerts[0].alert_id,
            reviewer_id: Uuid::now_v7(),
            decision: AmlAlertDecision::ClosedFalsePositive,
            _notes: Some("False positive".into()),
        }).await.unwrap();

        assert_eq!(result.alert.status, AlertStatus::Closed);
    }
}

//! Integration tests for compliance-service.

#[cfg(test)]
mod integration_tests {
    use uuid::Uuid;
    use crate::commands::{
        ComplianceCommandHandler, SubmitKybEvidence, ReviewKybCase,
        ScanTransaction, ReviewAmlAlert, AmlAlertDecision, CommandHandler,
    };
    use crate::queries::{ComplianceQueries, QueryHandler};
    use crate::domain::{KybStatus, AlertStatus};
    use crate::repository::InMemoryComplianceRepository;

    async fn setup() -> (ComplianceCommandHandler<InMemoryComplianceRepository>, ComplianceQueries<InMemoryComplianceRepository>) {
        let repo = InMemoryComplianceRepository::new();
        let handler = ComplianceCommandHandler::new(repo.clone());
        let queries = ComplianceQueries::new(repo);
        (handler, queries)
    }

    #[tokio::test]
    async fn test_full_kyb_lifecycle() {
        let (handler, queries) = setup().await;
        let operator_id = Uuid::now_v7();
        let submitted_by = Uuid::now_v7();
        let reviewer_id = Uuid::now_v7();

        // Submit KYB
        let submitted = handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id,
            document_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            submitted_by,
        }).await.unwrap();

        assert_eq!(submitted.kyb_case.status, KybStatus::Submitted);

        // Query
        let case = queries.get_kyb_case(submitted.kyb_case.kyb_case_id).await.unwrap().unwrap();
        assert_eq!(case.operator_id, operator_id);

        // Approve
        let approved = handler.review_kyb_case(ReviewKybCase {
            kyb_case_id: submitted.kyb_case.kyb_case_id,
            approved: true,
            reason: None,
            reviewed_by: reviewer_id,
        }).await.unwrap();

        assert_eq!(approved.kyb_case.status, KybStatus::Approved);
    }

    #[tokio::test]
    async fn test_aml_scan_and_review_flow() {
        let (handler, queries) = setup().await;
        let operator_id = Uuid::now_v7();

        // Scan transaction
        let scan = handler.scan_transaction(ScanTransaction {
            transaction_id: Uuid::now_v7(),
            operator_id,
            amount_minor_units: 600_000_00, // 600,000 AED - triggers amount anomaly
            payment_method_id: Some("card_test_123".into()),
        }).await.unwrap();

        assert!(!scan.alerts.is_empty());
        assert!(scan.blocked); // critical severity blocks

        // List alerts
        let alerts = queries.list_aml_alerts(operator_id, Some("open")).await.unwrap();
        assert!(!alerts.is_empty());

        // Review alert
        let reviewed = handler.review_aml_alert(ReviewAmlAlert {
            alert_id: scan.alerts[0].alert_id,
            reviewer_id: Uuid::now_v7(),
            decision: AmlAlertDecision::ClosedFalsePositive,
            notes: Some("Legitimate high-value transaction".into()),
        }).await.unwrap();

        assert_eq!(reviewed.alert.status, AlertStatus::Closed);
    }

    #[tokio::test]
    async fn test_pending_kyb_cases_list() {
        let (handler, queries) = setup().await;

        let op1 = Uuid::now_v7();
        let op2 = Uuid::now_v7();

        handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: op1,
            document_ids: vec![Uuid::now_v7()],
            submitted_by: Uuid::now_v7(),
        }).await.unwrap();

        handler.submit_kyb_evidence(SubmitKybEvidence {
            operator_id: op2,
            document_ids: vec![Uuid::now_v7()],
            submitted_by: Uuid::now_v7(),
        }).await.unwrap();

        let pending = queries.list_pending_kyb_cases().await.unwrap();
        assert_eq!(pending.len(), 2);
    }
}

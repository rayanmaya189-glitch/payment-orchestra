//! Command handler tests for BC-03 Merchant Compliance.
//!
//! Per CONVENTIONS.md: tests extracted to their own file.
//! Gated by `#[cfg(test)] pub mod tests;` in commands/mod.rs.

use uuid::Uuid;

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

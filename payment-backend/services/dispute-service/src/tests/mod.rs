//! Dispute Management TDD tests — BC-10
//!
//! Spec test cases:
//! - test_record_chargeback_success: basic creation with received status
//! - test_record_chargeback_on_uncaptured_intent_rejected: INV-08
//! - test_submit_representment_requires_valid_evidence: validation
//! - test_submit_representment_success: full representment flow
//! - test_resolve_chargeback_won: won outcome
//! - test_resolve_chargeback_lost: lost outcome
//! - test_resolve_already_resolved_rejected: idempotency
//! - test_find_by_payment_intent: query
//! - test_find_open_cases: query

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> DisputePipeline {
    DisputePipeline::new()
}

async fn create_test_case(pipeline: &DisputePipeline, captured: bool) -> ChargebackCase {
    let cmd = RecordChargebackCommand {
        operator_id: Uuid::now_v7(),
        payment_intent_id: Uuid::now_v7(),
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "fraud".into(),
        amount_minor_units: 5000,
        currency: "AED".into(),
        is_captured: captured,
    };
    pipeline.api.record_chargeback(cmd).await.unwrap()
}

fn valid_evidence() -> RepresentmentEvidence {
    RepresentmentEvidence {
        transaction_receipt: Some(Uuid::now_v7()),
        delivery_confirmation: None,
        customer_communication: None,
        cardholder_agreement: None,
        refund_policy: None,
        description: "Customer received the product on time. See attached receipt.".into(),
        supporting_documents: vec![Uuid::now_v7()],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_record_chargeback_success() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    assert_eq!(case.status, ChargebackStatus::Received);
    assert_eq!(case.reason_code, "fraud");
    assert_eq!(case.amount_minor_units, 5000);
    assert_eq!(case.currency, "AED");
    assert!(case.representment_deadline > case.received_at);
}

#[tokio::test]
async fn test_record_chargeback_on_uncaptured_intent_rejected() {
    let pipeline = setup();
    let cmd = RecordChargebackCommand {
        operator_id: Uuid::now_v7(),
        payment_intent_id: Uuid::now_v7(),
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "fraud".into(),
        amount_minor_units: 5000,
        currency: "AED".into(),
        is_captured: false, // NOT captured
    };

    let result = pipeline.api.record_chargeback(cmd).await;
    assert!(result.is_err(), "Should reject chargeback on uncaptured intent");
}

#[tokio::test]
async fn test_submit_representment_requires_valid_evidence() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    // Empty description + no receipt should fail validation
    let invalid_evidence = RepresentmentEvidence {
        transaction_receipt: None,
        delivery_confirmation: None,
        customer_communication: None,
        cardholder_agreement: None,
        refund_policy: None,
        description: "".into(),
        supporting_documents: vec![Uuid::now_v7()],
    };

    let result = pipeline
        .api
        .submit_representment(SubmitRepresentmentCommand {
            chargeback_id: case.chargeback_id,
            evidence: invalid_evidence,
        })
        .await;

    assert!(result.is_err(), "Empty evidence should be rejected");
}

#[tokio::test]
async fn test_submit_representment_success() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    let updated = pipeline
        .api
        .submit_representment(SubmitRepresentmentCommand {
            chargeback_id: case.chargeback_id,
            evidence: valid_evidence(),
        })
        .await
        .unwrap();

    assert_eq!(updated.status, ChargebackStatus::RepresentmentSubmitted);
    assert_eq!(updated.submissions.len(), 1);
    assert_eq!(updated.submissions[0].evidence.description, "Customer received the product on time. See attached receipt.");
}

#[tokio::test]
async fn test_resolve_chargeback_won() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    // Submit representment first
    let _ = pipeline
        .api
        .submit_representment(SubmitRepresentmentCommand {
            chargeback_id: case.chargeback_id,
            evidence: valid_evidence(),
        })
        .await
        .unwrap();

    // Win the case
    let resolved = pipeline
        .api
        .resolve_chargeback(ResolveChargebackCommand {
            chargeback_id: case.chargeback_id,
            outcome: ChargebackOutcome::Won,
            resolution_note: Some("Acquirer accepted our evidence".into()),
        })
        .await
        .unwrap();

    assert_eq!(resolved.status, ChargebackStatus::Won);
    assert_eq!(resolved.outcome, Some(ChargebackOutcome::Won));
    assert!(resolved.resolved_at.is_some());
    assert!(resolved.submissions[0].response_received_at.is_some());
}

#[tokio::test]
async fn test_resolve_chargeback_lost() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    // Submit representment first (must be in proper state to resolve)
    let _ = pipeline
        .api
        .submit_representment(SubmitRepresentmentCommand {
            chargeback_id: case.chargeback_id,
            evidence: valid_evidence(),
        })
        .await
        .unwrap();

    // Then resolve as lost
    let resolved = pipeline
        .api
        .resolve_chargeback(ResolveChargebackCommand {
            chargeback_id: case.chargeback_id,
            outcome: ChargebackOutcome::Lost,
            resolution_note: None,
        })
        .await
        .unwrap();

    assert_eq!(resolved.status, ChargebackStatus::Lost);
    assert_eq!(resolved.outcome, Some(ChargebackOutcome::Lost));
}

#[tokio::test]
async fn test_resolve_already_resolved_rejected() {
    let pipeline = setup();
    let case = create_test_case(&pipeline, true).await;

    // Submit representment first
    let _ = pipeline
        .api
        .submit_representment(SubmitRepresentmentCommand {
            chargeback_id: case.chargeback_id,
            evidence: valid_evidence(),
        })
        .await
        .unwrap();

    // Resolve once
    pipeline
        .api
        .resolve_chargeback(ResolveChargebackCommand {
            chargeback_id: case.chargeback_id,
            outcome: ChargebackOutcome::Won,
            resolution_note: None,
        })
        .await
        .unwrap();

    // Resolve again should fail
    let result = pipeline
        .api
        .resolve_chargeback(ResolveChargebackCommand {
            chargeback_id: case.chargeback_id,
            outcome: ChargebackOutcome::Lost,
            resolution_note: None,
        })
        .await;
    assert!(result.is_err(), "Double resolve should fail");
}

#[tokio::test]
async fn test_find_by_payment_intent() {
    let pipeline = setup();
    let payment_intent_id = Uuid::now_v7();

    // Create two chargebacks for same payment intent
    let cmd1 = RecordChargebackCommand {
        operator_id: Uuid::now_v7(),
        payment_intent_id,
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "fraud".into(),
        amount_minor_units: 5000,
        currency: "AED".into(),
        is_captured: true,
    };
    pipeline.api.record_chargeback(cmd1).await.unwrap();

    let cmd2 = RecordChargebackCommand {
        operator_id: Uuid::now_v7(),
        payment_intent_id,
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "duplicate".into(),
        amount_minor_units: 3000,
        currency: "AED".into(),
        is_captured: true,
    };
    pipeline.api.record_chargeback(cmd2).await.unwrap();

    let results = pipeline
        .api
        .find_by_payment_intent(payment_intent_id)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_find_open_cases() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    // Create an open case
    let open_cmd = RecordChargebackCommand {
        operator_id,
        payment_intent_id: Uuid::now_v7(),
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "fraud".into(),
        amount_minor_units: 5000,
        currency: "AED".into(),
        is_captured: true,
    };
    let open_case = pipeline.api.record_chargeback(open_cmd).await.unwrap();

    // Create a resolved case (Accepted directly from Received is valid)
    let resolved_cmd = RecordChargebackCommand {
        operator_id,
        payment_intent_id: Uuid::now_v7(),
        acquirer_link_id: Uuid::now_v7(),
        reason_code: "duplicate".into(),
        amount_minor_units: 3000,
        currency: "AED".into(),
        is_captured: true,
    };
    let resolved_case = pipeline.api.record_chargeback(resolved_cmd).await.unwrap();
    pipeline
        .api
        .resolve_chargeback(ResolveChargebackCommand {
            chargeback_id: resolved_case.chargeback_id,
            outcome: ChargebackOutcome::Accepted, // Received → Accepted is valid
            resolution_note: None,
        })
        .await
        .unwrap();

    // Should only find 1 open case
    let open_cases = pipeline.api.find_open_cases(operator_id).await.unwrap();
    assert_eq!(open_cases.len(), 1);
    assert_eq!(open_cases[0].chargeback_id, open_case.chargeback_id);
}

#[tokio::test]
async fn test_query_nonexistent_chargeback() {
    let pipeline = setup();
    let result = pipeline.api.get_chargeback(Uuid::now_v7()).await;
    assert!(result.is_err(), "Nonexistent chargeback should error");
}

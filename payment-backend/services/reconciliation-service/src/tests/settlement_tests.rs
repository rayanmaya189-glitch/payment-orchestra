//! Settlement tests: batch ingestion and matching.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;
use crate::repository::*;

use super::{setup_handler, make_settlement_record};

#[tokio::test]
async fn test_ingest_settlement_batch_success() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();
    let acquirer_link_id = Uuid::now_v7();

    let result = handler.ingest_settlement_batch(IngestSettlementBatch {
        operator_id,
        acquirer_link_id,
        raw_file: b"test settlement data".to_vec(),
        file_format: SettlementFormat::Csv,
    }).await.unwrap();

    assert!(result.total_records > 0);
    assert_eq!(result.events.len(), 1);
}

#[tokio::test]
async fn test_ingest_duplicate_batch_rejected() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();
    let acquirer_link_id = Uuid::now_v7();
    let raw = b"test settlement data".to_vec();

    handler.ingest_settlement_batch(IngestSettlementBatch {
        operator_id,
        acquirer_link_id,
        raw_file: raw.clone(),
        file_format: SettlementFormat::Csv,
    }).await.unwrap();

    let result = handler.ingest_settlement_batch(IngestSettlementBatch {
        operator_id,
        acquirer_link_id,
        raw_file: raw,
        file_format: SettlementFormat::Csv,
    }).await;

    assert!(matches!(result, Err(ReconciliationError::DuplicateBatch(_))));
}

#[tokio::test]
async fn test_exact_match_produces_auto_confirm() {
    let matcher = ReconciliationMatcher::default();
    let pi_id = Uuid::now_v7();
    let record = SettlementRecord {
        record_id: Uuid::now_v7(),
        settlement_batch_id: Uuid::now_v7(),
        transaction_id: "txn_001".into(),
        acquirer_reference: Some("acq_ref_001".into()),
        amount_minor: 10000,
        currency: "AED".into(),
        fee_minor: Some(250),
        settlement_date: None,
        status: "pending".into(),
        match_outcome: None,
        matched_payment_intent_id: None,
    };

    let payment_intents = vec![PaymentIntentRef {
        payment_intent_id: pi_id,
        acquirer_reference: Some("acq_ref_001".into()),
        amount_minor: 10000,
        currency: "AED".into(),
    }];

    let result = matcher.match_record(&record, &payment_intents);
    assert_eq!(result.outcome, SettlementMatchOutcome::AutoConfirmed);
    assert!((result.confidence - 1.0).abs() < 0.001);
    assert_eq!(result.payment_intent_id, Some(pi_id));
}

#[tokio::test]
async fn test_unmatched_record_flagged() {
    let matcher = ReconciliationMatcher::default();
    let record = SettlementRecord {
        record_id: Uuid::now_v7(),
        settlement_batch_id: Uuid::now_v7(),
        transaction_id: "txn_002".into(),
        acquirer_reference: None,
        amount_minor: 10000,
        currency: "AED".into(),
        fee_minor: None,
        settlement_date: None,
        status: "pending".into(),
        match_outcome: None,
        matched_payment_intent_id: None,
    };

    // No matching payment intents
    let result = matcher.match_record(&record, &[]);
    assert_eq!(result.outcome, SettlementMatchOutcome::Unmatched);
    assert!(result.payment_intent_id.is_none());
}

#[tokio::test]
async fn test_amount_mismatch_record_flagged() {
    let matcher = ReconciliationMatcher::default();
    let pi_id = Uuid::now_v7();
    let record = SettlementRecord {
        record_id: Uuid::now_v7(),
        settlement_batch_id: Uuid::now_v7(),
        transaction_id: "txn_003".into(),
        acquirer_reference: Some("acq_ref_003".into()),
        amount_minor: 5000, // Different from captured
        currency: "AED".into(),
        fee_minor: None,
        settlement_date: None,
        status: "pending".into(),
        match_outcome: None,
        matched_payment_intent_id: None,
    };

    let payment_intents = vec![PaymentIntentRef {
        payment_intent_id: pi_id,
        acquirer_reference: Some("acq_ref_003".into()),
        amount_minor: 10000, // Captured different amount
        currency: "AED".into(),
    }];

    let result = matcher.match_record(&record, &payment_intents);
    assert_eq!(result.outcome, SettlementMatchOutcome::AmountMismatch);
}

#[tokio::test]
async fn test_duplicate_reference_flagged() {
    let matcher = ReconciliationMatcher::default();
    let record = SettlementRecord {
        record_id: Uuid::now_v7(),
        settlement_batch_id: Uuid::now_v7(),
        transaction_id: "txn_004".into(),
        acquirer_reference: Some("acq_ref_shared".into()),
        amount_minor: 10000,
        currency: "AED".into(),
        fee_minor: None,
        settlement_date: None,
        status: "pending".into(),
        match_outcome: None,
        matched_payment_intent_id: None,
    };

    // Two PaymentIntents with same acquirer_reference
    let payment_intents = vec![
        PaymentIntentRef {
            payment_intent_id: Uuid::now_v7(),
            acquirer_reference: Some("acq_ref_shared".into()),
            amount_minor: 10000,
            currency: "AED".into(),
        },
        PaymentIntentRef {
            payment_intent_id: Uuid::now_v7(),
            acquirer_reference: Some("acq_ref_shared".into()),
            amount_minor: 10000,
            currency: "AED".into(),
        },
    ];

    let result = matcher.match_record(&record, &payment_intents);
    assert_eq!(result.outcome, SettlementMatchOutcome::DuplicateReference);
}

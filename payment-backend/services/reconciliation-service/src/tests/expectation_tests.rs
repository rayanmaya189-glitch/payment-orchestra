//! Settlement expectation tests.

use uuid::Uuid;
use chrono::Utc;

use crate::domain::*;
use crate::commands::*;
use crate::queries::*;
use crate::events::ReconciliationEvent;
use crate::repository::*;

use super::{setup_handler};

#[tokio::test]
async fn test_create_settlement_expectation() {
    let (handler, repo) = setup_handler();
    let pi_id = Uuid::now_v7();
    let link_id = Uuid::now_v7();
    let expected_date = Utc::now();

    let event = handler.create_settlement_expectation(CreateSettlementExpectation {
        payment_intent_id: pi_id,
        acquirer_link_id: link_id,
        expected_settlement_date: expected_date,
        settlement_cycle: "next_day".into(),
    }).await.unwrap();

    match event {
        ReconciliationEvent::SettlementExpected(e) => {
            assert_eq!(e.payment_intent_id, pi_id);
        }
        _ => panic!("Expected SettlementExpected event"),
    }

    let found = repo.find_expectation_by_payment(pi_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().status, ExpectationStatus::Pending);
}

#[tokio::test]
async fn test_query_settlement_batch() {
    let repo = InMemoryReconciliationRepository::new();
    let handler = ReconciliationCommandHandler::new(repo.clone());
    let query_handler = ReconciliationQueryHandler::new(repo.clone());
    let operator_id = Uuid::now_v7();
    let link_id = Uuid::now_v7();

    let ingest = handler.ingest_settlement_batch(IngestSettlementBatch {
        operator_id,
        acquirer_link_id: link_id,
        raw_file: b"test data".to_vec(),
        file_format: SettlementFormat::Csv,
    }).await.unwrap();

    let loaded = query_handler.get_settlement_batch(GetSettlementBatchQuery {
        settlement_batch_id: ingest.settlement_batch_id,
    }).await.unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().operator_id, operator_id);
}

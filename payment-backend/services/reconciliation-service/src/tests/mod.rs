//! Comprehensive TDD tests for reconciliation-service.

mod expectation_tests;
mod fee_tests;
mod ledger_tests;
mod settlement_tests;

pub(crate) mod helpers {
    use uuid::Uuid;

    use crate::domain::*;
    use crate::commands::*;
    use crate::repository::*;

    pub fn setup_handler() -> (ReconciliationCommandHandler<InMemoryReconciliationRepository>, InMemoryReconciliationRepository) {
        let repo = InMemoryReconciliationRepository::new();
        let handler = ReconciliationCommandHandler::new(repo.clone());
        (handler, repo)
    }

    pub fn make_settlement_record(batch_id: Uuid, transaction_id: &str, amount: i64) -> SettlementRecord {
        SettlementRecord {
            record_id: Uuid::now_v7(),
            settlement_batch_id: batch_id,
            transaction_id: transaction_id.to_string(),
            acquirer_reference: Some(format!("acq_ref_{}", transaction_id)),
            amount_minor: amount,
            currency: "AED".into(),
            fee_minor: None,
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        }
    }
}

pub(crate) use helpers::*;

//! Reconciliation-service domain model — settlement matching engine.
//!
//! Event-sourced aggregates: SettlementBatch, LedgerEntry.
//! CRUD + events aggregates: SettlementExpectation, FeeVariance.
//!
//! File structure (one concept per file per CONVENTIONS.md):
//!
//! - [`error`]                  — [`ReconciliationError`]
//! - [`money`]                  — [`Money`] value object
//! - [`types`]                  — small shared enums ([`BatchStatus`], [`EntryType`], [`ExpectationStatus`], [`FeeVarianceStatus`], [`SettlementFormat`], [`SettlementFileChecksum`])
//! - [`match_result`]           — [`MatchResult`], [`SettlementMatchOutcome`], [`MatchStrategy`], [`PaymentIntentRef`]
//! - [`settlement_batch`]       — [`SettlementBatch`], [`SettlementRecord`] (AGG-01)
//! - [`ledger_entry`]           — [`LedgerEntry`] (AGG-02)
//! - [`settlement_expectation`] — [`SettlementExpectation`] (AGG-03)
//! - [`fee_variance`]           — [`FeeVariance`] (AGG-04)
//! - [`matcher`]                — [`ReconciliationMatcher`]

pub mod error;
pub mod fee_variance;
pub mod ledger_entry;
pub mod match_result;
pub mod matcher;
pub mod money;
pub mod settlement_batch;
pub mod settlement_expectation;
pub mod types;

pub use error::*;
pub use fee_variance::*;
pub use ledger_entry::*;
pub use match_result::*;
pub use matcher::*;
pub use money::*;
pub use settlement_batch::*;
pub use settlement_expectation::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_money_zero() {
        let m = Money::zero("USD");
        assert_eq!(m.amount_minor_units, 0);
        assert_eq!(m.currency, "USD");
    }

    #[test]
    fn test_money_checked_add() {
        let a = Money {
            amount_minor_units: 100,
            currency: "USD".into(),
        };
        let b = Money {
            amount_minor_units: 50,
            currency: "USD".into(),
        };
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.amount_minor_units, 150);
    }

    #[test]
    fn test_money_checked_add_currency_mismatch() {
        let a = Money {
            amount_minor_units: 100,
            currency: "USD".into(),
        };
        let b = Money {
            amount_minor_units: 50,
            currency: "EUR".into(),
        };
        assert!(a.checked_add(&b).is_err());
    }

    #[test]
    fn test_money_checked_sub() {
        let a = Money {
            amount_minor_units: 100,
            currency: "USD".into(),
        };
        let b = Money {
            amount_minor_units: 30,
            currency: "USD".into(),
        };
        let diff = a.checked_sub(&b).unwrap();
        assert_eq!(diff.amount_minor_units, 70);
    }

    #[test]
    fn test_money_checked_sub_insufficient() {
        let a = Money {
            amount_minor_units: 30,
            currency: "USD".into(),
        };
        let b = Money {
            amount_minor_units: 100,
            currency: "USD".into(),
        };
        assert!(a.checked_sub(&b).is_err());
    }

    #[test]
    fn test_batch_status_display() {
        assert_eq!(format!("{}", BatchStatus::Ingesting), "pending");
        assert_eq!(format!("{}", BatchStatus::Processed), "matched");
        assert_eq!(format!("{}", BatchStatus::Quarantined), "exception");
    }

    #[test]
    fn test_settlement_batch_new() {
        let record = SettlementRecord {
            record_id: Uuid::now_v7(),
            settlement_batch_id: Uuid::now_v7(),
            transaction_id: "txn_001".into(),
            acquirer_reference: Some("acq_ref_001".into()),
            amount_minor: 1000,
            currency: "USD".into(),
            fee_minor: Some(30),
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        };

        let batch = SettlementBatch::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "sha256-checksum".into(),
            "csv".into(),
            vec![record],
        );

        assert_eq!(batch.status, BatchStatus::Ingesting);
        assert_eq!(batch.total_records, 1);
        assert_eq!(batch.total_amount_minor, 1000);
        assert!(batch.processed_at.is_none());
    }

    #[test]
    fn test_matcher_exact_match() {
        let matcher = ReconciliationMatcher::default();
        let record = SettlementRecord {
            record_id: Uuid::now_v7(),
            settlement_batch_id: Uuid::now_v7(),
            transaction_id: "txn_001".into(),
            acquirer_reference: Some("acq_ref_001".into()),
            amount_minor: 1000,
            currency: "USD".into(),
            fee_minor: Some(30),
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        };

        let payment_intents = vec![PaymentIntentRef {
            payment_intent_id: Uuid::now_v7(),
            acquirer_reference: Some("acq_ref_001".into()),
            amount_minor: 1000,
            currency: "USD".into(),
        }];

        let result = matcher.match_record(&record, &payment_intents);
        assert_eq!(result.strategy, MatchStrategy::Exact);
        assert_eq!(result.outcome, SettlementMatchOutcome::AutoConfirmed);
        assert_eq!(result.confidence, 1.0);
        assert!(result.payment_intent_id.is_some());
    }

    #[test]
    fn test_matcher_fuzzy_match() {
        let matcher = ReconciliationMatcher::default();
        let record = SettlementRecord {
            record_id: Uuid::now_v7(),
            settlement_batch_id: Uuid::now_v7(),
            transaction_id: "txn_002".into(),
            acquirer_reference: None, // No ref — forces fuzzy
            amount_minor: 950,
            currency: "USD".into(),
            fee_minor: None,
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        };

        let payment_intents = vec![PaymentIntentRef {
            payment_intent_id: Uuid::now_v7(),
            acquirer_reference: None,
            amount_minor: 1000,
            currency: "USD".into(),
        }];

        let result = matcher.match_record(&record, &payment_intents);
        assert_eq!(result.strategy, MatchStrategy::Fuzzy);
        // 950 vs 1000 → diff of 50, within 10% of 1000 (100)
        assert!(result.payment_intent_id.is_some());
    }

    #[test]
    fn test_matcher_unmatched() {
        let matcher = ReconciliationMatcher::default();
        let record = SettlementRecord {
            record_id: Uuid::now_v7(),
            settlement_batch_id: Uuid::now_v7(),
            transaction_id: "txn_003".into(),
            acquirer_reference: None,
            amount_minor: 5000,
            currency: "USD".into(),
            fee_minor: None,
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        };

        let payment_intents = vec![PaymentIntentRef {
            payment_intent_id: Uuid::now_v7(),
            acquirer_reference: None,
            amount_minor: 1000,
            currency: "USD".into(),
        }];

        let result = matcher.match_record(&record, &payment_intents);
        assert_eq!(result.outcome, SettlementMatchOutcome::Unmatched);
        assert!(result.payment_intent_id.is_none());
    }
}

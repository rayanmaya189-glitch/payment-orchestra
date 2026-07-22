//! Comprehensive TDD tests for reconciliation-service.

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use chrono::Utc;

    use crate::domain::*;
    use crate::commands::*;
    use crate::queries::*;
    use crate::repository::*;
    use crate::events::ReconciliationEvent;

    fn setup_handler() -> (ReconciliationCommandHandler<InMemoryReconciliationRepository>, InMemoryReconciliationRepository) {
        let repo = InMemoryReconciliationRepository::new();
        let handler = ReconciliationCommandHandler::new(repo.clone());
        (handler, repo)
    }

    fn make_settlement_record(batch_id: Uuid, transaction_id: &str, amount: i64) -> SettlementRecord {
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

    // ══════════════════════════════════════════════════════════════════════
    // IngestSettlementBatch Tests
    // ══════════════════════════════════════════════════════════════════════

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

    // ══════════════════════════════════════════════════════════════════════
    // Settle ment Matching Tests
    // ══════════════════════════════════════════════════════════════════════

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

    // ══════════════════════════════════════════════════════════════════════
    // Fee Variance Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_fee_variance_within_tolerance() {
        let (handler, _) = setup_handler();
        let pi_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let result = handler.track_fee_variance(TrackFeeVariance {
            payment_intent_id: pi_id,
            acquirer_link_id: link_id,
            estimated_fee_minor: 250,
            actual_fee_minor: 260, // 4% difference, within 5% tolerance
            tolerance_threshold_percent: 5.0,
        }).await.unwrap();

        assert!(result.is_within_tolerance);
    }

    #[tokio::test]
    async fn test_fee_variance_detected() {
        let (handler, _) = setup_handler();
        let pi_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let result = handler.track_fee_variance(TrackFeeVariance {
            payment_intent_id: pi_id,
            acquirer_link_id: link_id,
            estimated_fee_minor: 250,
            actual_fee_minor: 300, // 20% difference, exceeds 5% tolerance
            tolerance_threshold_percent: 5.0,
        }).await.unwrap();

        assert!(!result.is_within_tolerance);
    }

    #[tokio::test]
    async fn test_resolve_fee_variance() {
        let (handler, repo) = setup_handler();
        let pi_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let result = handler.track_fee_variance(TrackFeeVariance {
            payment_intent_id: pi_id,
            acquirer_link_id: link_id,
            estimated_fee_minor: 250,
            actual_fee_minor: 300,
            tolerance_threshold_percent: 5.0,
        }).await.unwrap();

        // Resolve the variance
        let event = handler.resolve_fee_variance(ResolveFeeVarianceCmd {
            variance_id: result.variance_id,
            resolution: "Acquirer confirmed billing correct".into(),
        }).await.unwrap();

        match event {
            ReconciliationEvent::FeeVarianceResolved(e) => {
                assert_eq!(e.variance_id, result.variance_id);
            }
            _ => panic!("Expected FeeVarianceResolved event"),
        }

        // Verify the variance was updated
        let variance = repo.load_fee_variance(result.variance_id).await.unwrap().unwrap();
        assert!(variance.resolved_at.is_some());
    }

    // ══════════════════════════════════════════════════════════════════════
    // Ledger Balance Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_ledger_debit_credit_balanced() {
        let repo = InMemoryReconciliationRepository::new();
        let txn_id = Uuid::now_v7();

        // Add debit and credit entries
        repo.append_ledger_entry(&LedgerEntry {
            entry_id: Uuid::now_v7(),
            transaction_id: txn_id,
            entry_type: EntryType::Debit,
            amount_minor: 10000,
            currency: "AED".into(),
            source_acquirer: "mock_acquirer".into(),
            reconciliation_batch_id: None,
            reconciled: false,
            created_at: Utc::now(),
        }).await.unwrap();

        repo.append_ledger_entry(&LedgerEntry {
            entry_id: Uuid::now_v7(),
            transaction_id: txn_id,
            entry_type: EntryType::Credit,
            amount_minor: 10000,
            currency: "AED".into(),
            source_acquirer: "mock_acquirer".into(),
            reconciliation_batch_id: None,
            reconciled: false,
            created_at: Utc::now(),
        }).await.unwrap();

        let balanced = repo.verify_balance(txn_id).await.unwrap();
        assert!(balanced, "Debit and credit should be balanced");
    }

    #[tokio::test]
    async fn test_ledger_imbalanced_detected() {
        let repo = InMemoryReconciliationRepository::new();
        let txn_id = Uuid::now_v7();

        // Add only a debit (no matching credit)
        repo.append_ledger_entry(&LedgerEntry {
            entry_id: Uuid::now_v7(),
            transaction_id: txn_id,
            entry_type: EntryType::Debit,
            amount_minor: 10000,
            currency: "AED".into(),
            source_acquirer: "mock_acquirer".into(),
            reconciliation_batch_id: None,
            reconciled: false,
            created_at: Utc::now(),
        }).await.unwrap();

        let balanced = repo.verify_balance(txn_id).await.unwrap();
        assert!(!balanced, "Debit without matching credit should be imbalanced");
    }

    // ══════════════════════════════════════════════════════════════════════
    // Settlement Expectation Tests
    // ══════════════════════════════════════════════════════════════════════

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

    // ══════════════════════════════════════════════════════════════════════
    // Query Tests
    // ══════════════════════════════════════════════════════════════════════

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
}

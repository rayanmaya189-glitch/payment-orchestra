use chrono::{DateTime, Utc};
use uuid::Uuid;

use shared_types::Money;
use crate::domain::value_objects::{MatchOutcome, SettlementStatus};

/// Settlement batch aggregate root.
///
/// Tracks the lifecycle of a batch of settlement records from a connector:
/// Pending → Polled → Matched → Settled (or Exception).
#[derive(Debug, Clone)]
pub struct SettlementBatch {
    pub batch_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub status: SettlementStatus,
    pub total_amount: Money,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub exception_count: i32,
    pub period_start: String,
    pub period_end: String,
    pub exceptions: Option<serde_json::Value>,
    pub polled_at: Option<DateTime<Utc>>,
    pub matched_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SettlementBatch {
    pub fn new(
        operator_id: Uuid,
        connector_id: String,
        period_start: String,
        period_end: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            batch_id: Uuid::now_v7(),
            operator_id,
            connector_id,
            status: SettlementStatus::Pending,
            total_amount: Money {
                amount_minor_units: 0,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            },
            total_records: 0,
            matched_count: 0,
            unmatched_count: 0,
            exception_count: 0,
            period_start,
            period_end,
            exceptions: None,
            polled_at: None,
            matched_at: None,
            settled_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to Polled after fetching records from the connector.
    pub fn mark_polled(&mut self, records: i32, total_amount: i64) {
        self.status = SettlementStatus::Polled;
        self.total_records = records;
        self.total_amount.amount_minor_units = total_amount;
        self.polled_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to Matched after running the matching algorithm.
    pub fn mark_matched(&mut self, matched: i32, unmatched: i32) {
        self.status = SettlementStatus::Matched;
        self.matched_count = matched;
        self.unmatched_count = unmatched;
        self.matched_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to Settled after all records confirmed.
    pub fn mark_settled(&mut self) {
        self.status = SettlementStatus::Settled;
        self.settled_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Transition to Exception when unreconcilable discrepancies exist.
    pub fn mark_exception(&mut self, exceptions: serde_json::Value) {
        self.status = SettlementStatus::Exception;
        self.exceptions = Some(exceptions);
        self.updated_at = Utc::now();
    }
}

/// Ledger entry aggregate (append-only).
///
/// Represents a single debit or credit entry in the internal ledger.
/// INV-10: SUM(debit) - SUM(credit) = 0 per transaction_id.
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub debit_amount_minor_units: i64,
    pub credit_amount_minor_units: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub fee: Option<Money>,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
}

impl LedgerEntry {
    pub fn new(
        transaction_id: Uuid,
        entry_type: &str,
        debit: i64,
        credit: i64,
        currency: &str,
        source_acquirer: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            entry_id: Uuid::now_v7(),
            transaction_id,
            entry_type: entry_type.to_string(),
            debit_amount_minor_units: debit,
            credit_amount_minor_units: credit,
            currency: currency.to_string(),
            source_acquirer: source_acquirer.to_string(),
            fee: None,
            reconciliation_batch_id: None,
            reconciled: false,
            created_at: now,
        }
    }

    /// Net amount: credit - debit. Positive means incoming funds.
    pub fn net_amount(&self) -> i64 {
        self.credit_amount_minor_units - self.debit_amount_minor_units
    }
}

// ==================== Settlement Matching Algorithm ====================

/// The core settlement matching engine.
///
/// Compares internal ledger entries against connector settlement records.
/// Strategy (per SRS §3.2):
/// 1. Exact match on acquirer_reference → Matched (confidence 1.0)
/// 2. No match → Unmatched
/// 3. Duplicate references → DuplicateReference
pub struct SettlementMatcher;

impl SettlementMatcher {
    /// Match a batch of settlement records against internal ledger entries.
    ///
    /// Returns a vector of `MatchResult` with outcome and confidence for each record.
    pub fn match_batch(
        settlement_records: &[crate::domain::value_objects::SettlementRecord],
        ledger_entries: &[crate::domain::aggregates::LedgerEntry],
    ) -> Vec<crate::domain::value_objects::MatchResult> {
        // Build a lookup map: acquirer_reference → Vec<LedgerEntry>
        let mut ledger_map: std::collections::HashMap<String, Vec<&LedgerEntry>> =
            std::collections::HashMap::new();
        for entry in ledger_entries {
            ledger_map
                .entry(entry.source_acquirer.clone())
                .or_default()
                .push(entry);
        }

        settlement_records
            .iter()
            .map(|record| Self::match_single(record, &ledger_map))
            .collect()
    }

    /// Match a single settlement record against the ledger entry map.
    fn match_single(
        record: &crate::domain::value_objects::SettlementRecord,
        ledger_map: &std::collections::HashMap<String, Vec<&LedgerEntry>>,
    ) -> crate::domain::value_objects::MatchResult {
        match ledger_map.get(&record.acquirer_reference) {
            None => crate::domain::value_objects::MatchResult {
                settlement: record.clone(),
                outcome: MatchOutcome::Unmatched,
                matched_entry: None,
                confidence: 0.0,
            },
            Some(entries) if entries.len() > 1 => {
                // INV-07: Matched record references exactly one PaymentIntent.
                // Multiple entries sharing the same acquirer_reference is a duplicate reference.
                crate::domain::value_objects::MatchResult {
                    settlement: record.clone(),
                    outcome: MatchOutcome::DuplicateReference,
                    matched_entry: None,
                    confidence: 0.0,
                }
            }
            Some(entries) => {
                let entry = entries[0];
                // Check amount match
                let amount_matches =
                    entry.net_amount() == record.amount.amount_minor_units;

                // Check fee if present on both sides
                let fee_matches = match (&entry.fee, &record.fee) {
                    (Some(e_fee), Some(r_fee)) => {
                        e_fee.amount_minor_units == r_fee.amount_minor_units
                    }
                    (None, None) => true,
                    _ => false,
                };

                if amount_matches && fee_matches {
                    crate::domain::value_objects::MatchResult {
                        settlement: record.clone(),
                        outcome: MatchOutcome::Matched,
                        matched_entry: Some(crate::domain::value_objects::LedgerEntryRecord {
                            entry_id: entry.entry_id,
                            transaction_id: entry.transaction_id,
                            acquirer_reference: entry.source_acquirer.clone(),
                            amount: Money {
                                amount_minor_units: entry.net_amount(),
                                currency: shared_types::CurrencyCode::new(&entry.currency)
                                    .unwrap_or_else(|_| shared_types::CurrencyCode::new("AED").unwrap()),
                            },
                            fee: entry.fee.clone(),
                            created_at: entry.created_at,
                        }),
                        confidence: 1.0,
                    }
                } else if !amount_matches {
                    crate::domain::value_objects::MatchResult {
                        settlement: record.clone(),
                        outcome: MatchOutcome::AmountMismatch,
                        matched_entry: Some(crate::domain::value_objects::LedgerEntryRecord {
                            entry_id: entry.entry_id,
                            transaction_id: entry.transaction_id,
                            acquirer_reference: entry.source_acquirer.clone(),
                            amount: Money {
                                amount_minor_units: entry.net_amount(),
                                currency: shared_types::CurrencyCode::new(&entry.currency)
                                    .unwrap_or_else(|_| shared_types::CurrencyCode::new("AED").unwrap()),
                            },
                            fee: entry.fee.clone(),
                            created_at: entry.created_at,
                        }),
                        confidence: 0.5,
                    }
                } else {
                    crate::domain::value_objects::MatchResult {
                        settlement: record.clone(),
                        outcome: MatchOutcome::FeeDiscrepancy,
                        matched_entry: Some(crate::domain::value_objects::LedgerEntryRecord {
                            entry_id: entry.entry_id,
                            transaction_id: entry.transaction_id,
                            acquirer_reference: entry.source_acquirer.clone(),
                            amount: Money {
                                amount_minor_units: entry.net_amount(),
                                currency: shared_types::CurrencyCode::new(&entry.currency)
                                    .unwrap_or_else(|_| shared_types::CurrencyCode::new("AED").unwrap()),
                            },
                            fee: entry.fee.clone(),
                            created_at: entry.created_at,
                        }),
                        confidence: 0.8,
                    }
                }
            }
        }
    }

    /// Compute batch-level statistics from match results.
    pub fn compute_stats(
        results: &[crate::domain::value_objects::MatchResult],
    ) -> BatchMatchStats {
        let mut stats = BatchMatchStats::default();
        for result in results {
            stats.total += 1;
            match result.outcome {
                MatchOutcome::Matched => stats.matched += 1,
                MatchOutcome::Unmatched => stats.unmatched += 1,
                MatchOutcome::AmountMismatch => {
                    stats.amount_mismatch += 1;
                    stats.exceptions.push(format!(
                        "Amount mismatch for {}: settlement={}, ledger={}",
                        result.settlement.acquirer_reference,
                        result.settlement.amount.amount_minor_units,
                        result
                            .matched_entry
                            .as_ref()
                            .map(|e| e.amount.amount_minor_units)
                            .unwrap_or(0),
                    ));
                }
                MatchOutcome::DuplicateReference => {
                    stats.duplicate_reference += 1;
                    stats.exceptions.push(format!(
                        "Duplicate reference: {}",
                        result.settlement.acquirer_reference,
                    ));
                }
                MatchOutcome::FeeDiscrepancy => {
                    stats.fee_discrepancy += 1;
                    stats.exceptions.push(format!(
                        "Fee discrepancy for {}",
                        result.settlement.acquirer_reference,
                    ));
                }
            }
        }
        stats
    }
}

#[derive(Debug, Clone, Default)]
pub struct BatchMatchStats {
    pub total: i32,
    pub matched: i32,
    pub unmatched: i32,
    pub amount_mismatch: i32,
    pub duplicate_reference: i32,
    pub fee_discrepancy: i32,
    pub exceptions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{LedgerEntryRecord, SettlementRecord};
    use shared_types::{CurrencyCode, Money};

    fn make_settlement(ref_: &str, amount: i64) -> SettlementRecord {
        SettlementRecord {
            acquirer_reference: ref_.to_string(),
            amount: Money {
                amount_minor_units: amount,
                currency: CurrencyCode::new("AED").unwrap(),
            },
            settled_at: Utc::now(),
            fee: Some(Money {
                amount_minor_units: 100,
                currency: CurrencyCode::new("AED").unwrap(),
            }),
            connector_id: "ni".to_string(),
            status: "settled".to_string(),
        }
    }

    fn make_ledger_entry(ref_: &str, credit: i64, debit: i64) -> LedgerEntry {
        let mut entry = LedgerEntry::new(
            Uuid::now_v7(),
            "settlement",
            debit,
            credit,
            "AED",
            ref_,
        );
        entry.fee = Some(Money {
            amount_minor_units: 100,
            currency: CurrencyCode::new("AED").unwrap(),
        });
        entry
    }

    // ==================== Aggregate Tests ====================

    #[test]
    fn test_new_batch() {
        let b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        assert_eq!(b.status, SettlementStatus::Pending);
        assert_eq!(b.total_records, 0);
    }

    #[test]
    fn test_mark_polled() {
        let mut b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        b.mark_polled(10, 100000);
        assert_eq!(b.status, SettlementStatus::Polled);
        assert_eq!(b.total_records, 10);
        assert_eq!(b.total_amount.amount_minor_units, 100000);
        assert!(b.polled_at.is_some());
    }

    #[test]
    fn test_mark_matched() {
        let mut b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        b.mark_polled(10, 100000);
        b.mark_matched(8, 2);
        assert_eq!(b.status, SettlementStatus::Matched);
        assert_eq!(b.matched_count, 8);
        assert_eq!(b.unmatched_count, 2);
    }

    #[test]
    fn test_mark_settled() {
        let mut b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        b.mark_polled(10, 100000);
        b.mark_matched(10, 0);
        b.mark_settled();
        assert_eq!(b.status, SettlementStatus::Settled);
        assert!(b.settled_at.is_some());
    }

    #[test]
    fn test_mark_exception() {
        let mut b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        b.mark_polled(10, 100000);
        let exceptions = serde_json::json!(["Mismatch on ref_001"]);
        b.mark_exception(exceptions.clone());
        assert_eq!(b.status, SettlementStatus::Exception);
        assert_eq!(b.exceptions, Some(exceptions));
    }

    // ==================== LedgerEntry Tests ====================

    #[test]
    fn test_ledger_entry_new() {
        let entry = LedgerEntry::new(Uuid::now_v7(), "settlement", 0, 10000, "AED", "NI_001");
        assert_eq!(entry.net_amount(), 10000);
        assert!(!entry.reconciled);
    }

    #[test]
    fn test_ledger_entry_net_amount() {
        let entry = LedgerEntry::new(Uuid::now_v7(), "refund", 5000, 0, "AED", "NI_002");
        assert_eq!(entry.net_amount(), -5000);
    }

    // ==================== Matching Algorithm Tests ====================

    #[test]
    fn test_exact_match() {
        let settlements = vec![make_settlement("NI_001", 10000)];
        let ledger = vec![make_ledger_entry("NI_001", 10000, 0)];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MatchOutcome::Matched);
        assert_eq!(results[0].confidence, 1.0);
        assert!(results[0].matched_entry.is_some());
    }

    #[test]
    fn test_unmatched_record() {
        let settlements = vec![make_settlement("NI_999", 10000)];
        let ledger = vec![make_ledger_entry("NI_001", 10000, 0)];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MatchOutcome::Unmatched);
        assert_eq!(results[0].confidence, 0.0);
    }

    #[test]
    fn test_amount_mismatch() {
        let settlements = vec![make_settlement("NI_001", 15000)]; // Different amount
        let ledger = vec![make_ledger_entry("NI_001", 10000, 0)];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MatchOutcome::AmountMismatch);
        assert_eq!(results[0].confidence, 0.5);
    }

    #[test]
    fn test_duplicate_reference() {
        let settlements = vec![make_settlement("NI_001", 10000)];
        // Two ledger entries with same acquirer_reference
        let ledger = vec![
            make_ledger_entry("NI_001", 10000, 0),
            make_ledger_entry("NI_001", 5000, 0),
        ];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MatchOutcome::DuplicateReference);
    }

    #[test]
    fn test_multiple_settlements() {
        let settlements = vec![
            make_settlement("NI_001", 10000),
            make_settlement("NI_002", 5000),
            make_settlement("NI_003", 8000),
        ];
        let ledger = vec![
            make_ledger_entry("NI_001", 10000, 0),
            make_ledger_entry("NI_002", 5000, 0),
        ];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].outcome, MatchOutcome::Matched);
        assert_eq!(results[1].outcome, MatchOutcome::Matched);
        assert_eq!(results[2].outcome, MatchOutcome::Unmatched);
    }

    #[test]
    fn test_compute_stats() {
        let settlements = vec![
            make_settlement("NI_001", 10000),
            make_settlement("NI_002", 5000),
            make_settlement("NI_003", 8000),
            make_settlement("NI_004", 3000),
        ];
        let ledger = vec![
            make_ledger_entry("NI_001", 10000, 0),
            make_ledger_entry("NI_002", 9999, 0), // amount mismatch
        ];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        let stats = SettlementMatcher::compute_stats(&results);

        assert_eq!(stats.total, 4);
        assert_eq!(stats.matched, 1);
        assert_eq!(stats.amount_mismatch, 1);
        assert_eq!(stats.unmatched, 2);
        assert_eq!(stats.exceptions.len(), 1); // NI_002 amount mismatch
    }

    #[test]
    fn test_empty_settlements() {
        let results = SettlementMatcher::match_batch(&[], &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_ledger() {
        let settlements = vec![make_settlement("NI_001", 10000)];
        let results = SettlementMatcher::match_batch(&settlements, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MatchOutcome::Unmatched);
    }

    #[test]
    fn test_batch_stats_all_matched() {
        let settlements = vec![
            make_settlement("NI_001", 10000),
            make_settlement("NI_002", 5000),
        ];
        let ledger = vec![
            make_ledger_entry("NI_001", 10000, 0),
            make_ledger_entry("NI_002", 5000, 0),
        ];

        let results = SettlementMatcher::match_batch(&settlements, &ledger);
        let stats = SettlementMatcher::compute_stats(&results);
        assert_eq!(stats.matched, 2);
        assert_eq!(stats.unmatched, 0);
        assert!(stats.exceptions.is_empty());
    }
}

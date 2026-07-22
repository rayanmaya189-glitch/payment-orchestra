//! Reconciliation-service domain model — settlement matching engine.
//! Event-sourced aggregates: SettlementBatch, LedgerEntry.
//! CRUD + events aggregates: SettlementExpectation, FeeVariance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Value Objects ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

impl Money {
    pub fn zero(currency: &str) -> Self {
        Self { amount_minor_units: 0, currency: currency.to_string() }
    }

    pub fn checked_add(&self, other: &Money) -> Result<Self, ReconciliationError> {
        if self.currency != other.currency {
            return Err(ReconciliationError::Validation("Currency mismatch".into()));
        }
        let sum = self.amount_minor_units.checked_add(other.amount_minor_units)
            .ok_or_else(|| ReconciliationError::Validation("Amount overflow".into()))?;
        Ok(Self { amount_minor_units: sum, currency: self.currency.clone() })
    }

    pub fn checked_sub(&self, other: &Money) -> Result<Self, ReconciliationError> {
        if self.currency != other.currency {
            return Err(ReconciliationError::Validation("Currency mismatch".into()));
        }
        if self.amount_minor_units < other.amount_minor_units {
            return Err(ReconciliationError::Validation("Insufficient amount".into()));
        }
        Ok(Self { amount_minor_units: self.amount_minor_units - other.amount_minor_units, currency: self.currency.clone() })
    }
}

/// SHA-256 checksum of settlement file content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementFileChecksum(pub String);

/// Settlement file format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettlementFormat {
    Webhook,
    PollingApi,
    Sftp,
    Csv,
    ScannedDocument,
}

/// Entry type for double-entry ledger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryType {
    Debit,
    Credit,
}

// ─── Settlement Match Outcome ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchResult {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Option<Uuid>,
    pub confidence: f64,
    pub strategy: MatchStrategy,
    pub outcome: SettlementMatchOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementMatchOutcome {
    AutoConfirmed,
    Matched,
    AmountMismatch,
    Unmatched,
    DuplicateReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchStrategy {
    Exact,
    Fuzzy,
    AiAssisted,
}

// ─── AGG-01: SettlementBatch (Aggregate Root, Event-Sourced) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBatch {
    pub settlement_batch_id: Uuid,
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub file_checksum: String,
    pub file_format: String,
    pub status: BatchStatus,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount_minor: i64,
    pub records: Vec<SettlementRecord>,
    pub ingested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchStatus {
    Ingesting,
    Processed,
    Quarantined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub record_id: Uuid,
    pub settlement_batch_id: Uuid,
    pub transaction_id: String,
    pub acquirer_reference: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub fee_minor: Option<i64>,
    pub settlement_date: Option<DateTime<Utc>>,
    pub status: String,
    pub match_outcome: Option<SettlementMatchOutcome>,
    pub matched_payment_intent_id: Option<Uuid>,
}

impl SettlementBatch {
    pub fn new(
        settlement_batch_id: Uuid,
        operator_id: Uuid,
        acquirer_link_id: Uuid,
        file_checksum: String,
        file_format: String,
        records: Vec<SettlementRecord>,
    ) -> Self {
        let total_minor: i64 = records.iter().map(|r| r.amount_minor).sum();
        Self {
            settlement_batch_id,
            operator_id,
            acquirer_link_id,
            file_checksum,
            file_format,
            status: BatchStatus::Ingesting,
            total_records: records.len() as i32,
            matched_count: 0,
            unmatched_count: 0,
            total_amount_minor: total_minor,
            records,
            ingested_at: Utc::now(),
            processed_at: None,
        }
    }
}

// ─── AGG-02: LedgerEntry (Append-Only) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: EntryType,
    pub amount_minor: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
}

// ─── AGG-03: SettlementExpectation ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpectation {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
    pub status: ExpectationStatus,
    pub settled_amount_minor: Option<i64>,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpectationStatus {
    Pending,
    Settled,
    Overdue,
    Adjusted,
}

// ─── AGG-04: FeeVariance ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVariance {
    pub variance_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub variance_minor: i64,
    pub variance_percent: f64,
    pub is_within_tolerance: bool,
    pub tolerance_threshold_percent: f64,
    pub status: FeeVarianceStatus,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeeVarianceStatus {
    WithinTolerance,
    VarianceDetected,
    Disputed,
    Resolved,
}

// ─── Reconciliation Matcher ──────────────────────────────────────────────────

pub struct ReconciliationMatcher {
    pub auto_confirm_threshold: f64,
    pub review_threshold: f64,
}

impl Default for ReconciliationMatcher {
    fn default() -> Self {
        Self { auto_confirm_threshold: 0.95, review_threshold: 0.70 }
    }
}

impl ReconciliationMatcher {
    pub fn match_record(
        &self,
        record: &SettlementRecord,
        payment_intents: &[PaymentIntentRef],
    ) -> MatchResult {
        let record_id = record.record_id;

        // Strategy 1: Exact match by acquirer_reference
        if let Some(ref acquirer_ref) = record.acquirer_reference {
            let exact_matches: Vec<&PaymentIntentRef> = payment_intents.iter()
                .filter(|pi| pi.acquirer_reference.as_deref() == Some(acquirer_ref.as_str()))
                .collect();

            match exact_matches.len() {
                0 => { /* fall through to fuzzy */ }
                1 => {
                    return MatchResult {
                        settlement_record_id: record_id,
                        payment_intent_id: Some(exact_matches[0].payment_intent_id),
                        confidence: 1.0,
                        strategy: MatchStrategy::Exact,
                        outcome: if exact_matches[0].amount_minor == record.amount_minor {
                            SettlementMatchOutcome::AutoConfirmed
                        } else {
                            SettlementMatchOutcome::AmountMismatch
                        },
                    };
                }
                _ => {
                    return MatchResult {
                        settlement_record_id: record_id,
                        payment_intent_id: None,
                        confidence: 0.0,
                        strategy: MatchStrategy::Exact,
                        outcome: SettlementMatchOutcome::DuplicateReference,
                    };
                }
            }
        }

        // Strategy 2: Fuzzy match by amount + date proximity
        let amount_matches: Vec<&PaymentIntentRef> = payment_intents.iter()
            .filter(|pi| {
                let amount_diff = (pi.amount_minor - record.amount_minor).abs();
                // Within fee tolerance: allow up to 10% difference for fee
                amount_diff <= (pi.amount_minor as f64 * 0.10) as i64
            })
            .collect();

        if amount_matches.len() == 1 {
            let confidence = 0.85;
            return MatchResult {
                settlement_record_id: record_id,
                payment_intent_id: Some(amount_matches[0].payment_intent_id),
                confidence,
                strategy: MatchStrategy::Fuzzy,
                outcome: if confidence >= self.auto_confirm_threshold {
                    SettlementMatchOutcome::AutoConfirmed
                } else if confidence >= self.review_threshold {
                    SettlementMatchOutcome::Matched
                } else {
                    SettlementMatchOutcome::Unmatched
                },
            };
        }

        // No match found
        MatchResult {
            settlement_record_id: record_id,
            payment_intent_id: None,
            confidence: 0.0,
            strategy: MatchStrategy::Fuzzy,
            outcome: SettlementMatchOutcome::Unmatched,
        }
    }
}

/// Lightweight payment intent reference for matching
#[derive(Debug, Clone)]
pub struct PaymentIntentRef {
    pub payment_intent_id: Uuid,
    pub acquirer_reference: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReconciliationError {
    #[error("Settlement batch not found: {0}")]
    NotFound(Uuid),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Duplicate batch: checksum {0} already ingested")]
    DuplicateBatch(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Ledger imbalance detected for transaction {0}")]
    LedgerImbalance(Uuid),
}

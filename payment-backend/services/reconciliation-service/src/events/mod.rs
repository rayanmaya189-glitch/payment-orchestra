//! Domain events for the reconciliation-service.
//! Settlement matching, fee variance tracking, and ledger events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All domain events emitted by the reconciliation service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconciliationEvent {
    // ── Settlement Batch Events ──────────────────────────────────────────
    SettlementBatchIngested(SettlementBatchIngested),
    SettlementRecordMatched(SettlementRecordMatched),
    SettlementRecordUnmatched(SettlementRecordUnmatched),
    ReconciliationExceptionResolved(ReconciliationExceptionResolved),

    // ── Ledger Events ───────────────────────────────────────────────────
    LedgerEntryCreated(LedgerEntryCreated),
    LedgerEntryReconciled(LedgerEntryReconciled),

    // ── Settlement Expectation Events ────────────────────────────────────
    SettlementExpected(SettlementExpected),
    SettlementOverdue(SettlementOverdue),
    SettlementCompleted(SettlementCompleted),

    // ── Fee Variance Events ─────────────────────────────────────────────
    FeeVarianceDetected(FeeVarianceDetected),
    FeeVarianceResolved(FeeVarianceResolved),
}

// ─── Settlement Batch Events ─────────────────────────────────────────────────

/// A settlement batch has been ingested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBatchIngested {
    pub settlement_batch_id: Uuid,
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub total_records: i32,
    pub total_amount_minor: i64,
    pub file_format: String,
    pub occurred_at: DateTime<Utc>,
}

/// A settlement record has been matched to a payment intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecordMatched {
    pub settlement_record_id: Uuid,
    pub settlement_batch_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_reference: String,
    pub matched_amount_minor: i64,
    pub fee_actual_minor: Option<i64>,
    pub confidence: f64,
    pub strategy: String,
    pub occurred_at: DateTime<Utc>,
}

/// A settlement record could not be matched
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecordUnmatched {
    pub settlement_record_id: Uuid,
    pub settlement_batch_id: Uuid,
    pub transaction_id: String,
    pub amount_minor: i64,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

/// A reconciliation exception has been resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationExceptionResolved {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
    pub occurred_at: DateTime<Utc>,
}

// ─── Ledger Events ───────────────────────────────────────────────────────────

/// A ledger entry has been created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntryCreated {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub amount_minor: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub occurred_at: DateTime<Utc>,
}

/// A ledger entry has been reconciled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntryReconciled {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub reconciled: bool,
    pub occurred_at: DateTime<Utc>,
}

// ─── Settlement Expectation Events ───────────────────────────────────────────

/// Settlement expected for a payment (created when payment is authorized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpected {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
    pub occurred_at: DateTime<Utc>,
}

/// Settlement is overdue (past expected date with no settlement record)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementOverdue {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub days_overdue: u32,
    pub occurred_at: DateTime<Utc>,
}

/// Settlement has been completed (matched and amount confirmed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementCompleted {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub settled_amount_minor: i64,
    pub settlement_date: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

// ─── Fee Variance Events ─────────────────────────────────────────────────────

/// A fee variance has been detected between estimated and actual fees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVarianceDetected {
    pub variance_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub variance_minor: i64,
    pub variance_percent: f64,
    pub is_within_tolerance: bool,
    pub occurred_at: DateTime<Utc>,
}

/// A fee variance has been resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVarianceResolved {
    pub variance_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
    pub occurred_at: DateTime<Utc>,
}

// ─── Event type constants ─────────────────────────────────────────────────────

pub mod event_types {
    pub const SETTLEMENT_BATCH_INGESTED: &str = "settlement.batch_ingested";
    pub const SETTLEMENT_RECORD_MATCHED: &str = "settlement.record_matched";
    pub const SETTLEMENT_RECORD_UNMATCHED: &str = "settlement.record_unmatched";
    pub const RECONCILIATION_EXCEPTION_RESOLVED: &str = "reconciliation.exception_resolved";
    pub const LEDGER_ENTRY_CREATED: &str = "ledger.entry_created";
    pub const LEDGER_ENTRY_RECONCILED: &str = "ledger.entry_reconciled";
    pub const SETTLEMENT_EXPECTED: &str = "settlement.expected";
    pub const SETTLEMENT_OVERDUE: &str = "settlement.overdue";
    pub const SETTLEMENT_COMPLETED: &str = "settlement.completed";
    pub const FEE_VARIANCE_DETECTED: &str = "fee.variance_detected";
    pub const FEE_VARIANCE_RESOLVED: &str = "fee.variance_resolved";
}

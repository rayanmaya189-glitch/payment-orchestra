//! Event type definitions for reconciliation-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Event type constants ────────────────────────────────────────────────────

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

impl ReconciliationEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SettlementBatchIngested(_) => SETTLEMENT_BATCH_INGESTED,
            Self::SettlementRecordMatched(_) => SETTLEMENT_RECORD_MATCHED,
            Self::SettlementRecordUnmatched(_) => SETTLEMENT_RECORD_UNMATCHED,
            Self::ReconciliationExceptionResolved(_) => RECONCILIATION_EXCEPTION_RESOLVED,
            Self::LedgerEntryCreated(_) => LEDGER_ENTRY_CREATED,
            Self::LedgerEntryReconciled(_) => LEDGER_ENTRY_RECONCILED,
            Self::SettlementExpected(_) => SETTLEMENT_EXPECTED,
            Self::SettlementOverdue(_) => SETTLEMENT_OVERDUE,
            Self::SettlementCompleted(_) => SETTLEMENT_COMPLETED,
            Self::FeeVarianceDetected(_) => FEE_VARIANCE_DETECTED,
            Self::FeeVarianceResolved(_) => FEE_VARIANCE_RESOLVED,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::SettlementBatchIngested(e) => e.occurred_at,
            Self::SettlementRecordMatched(e) => e.occurred_at,
            Self::SettlementRecordUnmatched(e) => e.occurred_at,
            Self::ReconciliationExceptionResolved(e) => e.occurred_at,
            Self::LedgerEntryCreated(e) => e.occurred_at,
            Self::LedgerEntryReconciled(e) => e.occurred_at,
            Self::SettlementExpected(e) => e.occurred_at,
            Self::SettlementOverdue(e) => e.occurred_at,
            Self::SettlementCompleted(e) => e.occurred_at,
            Self::FeeVarianceDetected(e) => e.occurred_at,
            Self::FeeVarianceResolved(e) => e.occurred_at,
        }
    }
}

// ─── Event structs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconciliationEvent {
    SettlementBatchIngested(SettlementBatchIngested),
    SettlementRecordMatched(SettlementRecordMatched),
    SettlementRecordUnmatched(SettlementRecordUnmatched),
    ReconciliationExceptionResolved(ReconciliationExceptionResolved),
    LedgerEntryCreated(LedgerEntryCreated),
    LedgerEntryReconciled(LedgerEntryReconciled),
    SettlementExpected(SettlementExpected),
    SettlementOverdue(SettlementOverdue),
    SettlementCompleted(SettlementCompleted),
    FeeVarianceDetected(FeeVarianceDetected),
    FeeVarianceResolved(FeeVarianceResolved),
}

// ─── Settlement Batch Events ─────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecordUnmatched {
    pub settlement_record_id: Uuid,
    pub settlement_batch_id: Uuid,
    pub transaction_id: String,
    pub amount_minor: i64,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationExceptionResolved {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
    pub occurred_at: DateTime<Utc>,
}

// ─── Ledger Events ───────────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntryReconciled {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub reconciled: bool,
    pub occurred_at: DateTime<Utc>,
}

// ─── Settlement Expectation Events ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpected {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementOverdue {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub days_overdue: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementCompleted {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub settled_amount_minor: i64,
    pub settlement_date: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

// ─── Fee Variance Events ─────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVarianceResolved {
    pub variance_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
    pub occurred_at: DateTime<Utc>,
}

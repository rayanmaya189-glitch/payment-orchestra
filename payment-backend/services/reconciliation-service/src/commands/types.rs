//! Command type definitions for reconciliation-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::*;
use crate::events::ReconciliationEvent;

// ─── Command Input Structs ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IngestSettlementBatch {
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub raw_file: Vec<u8>,
    pub file_format: SettlementFormat,
}

#[derive(Debug, Clone)]
pub struct ProcessBatchMatching {
    pub settlement_batch_id: Uuid,
    pub payment_intents: Vec<PaymentIntentRef>,
}

#[derive(Debug, Clone)]
pub struct ResolveException {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
}

#[derive(Debug, Clone)]
pub struct TrackFeeVariance {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub tolerance_threshold_percent: f64,
}

#[derive(Debug, Clone)]
pub struct ResolveFeeVarianceCmd {
    pub variance_id: Uuid,
    pub resolution: String,
}

#[derive(Debug, Clone)]
pub struct CreateSettlementExpectation {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
}

// ─── Command Results ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestBatchResult {
    pub settlement_batch_id: Uuid,
    pub total_records: i32,
    pub total_amount_minor: i64,
    pub events: Vec<ReconciliationEvent>,
}

#[derive(Debug, Clone)]
pub struct MatchingResult {
    pub settlement_batch_id: Uuid,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub results: Vec<MatchResult>,
    pub events: Vec<ReconciliationEvent>,
}

#[derive(Debug, Clone)]
pub struct FeeVarianceResult {
    pub variance_id: Uuid,
    pub is_within_tolerance: bool,
    pub event: ReconciliationEvent,
}

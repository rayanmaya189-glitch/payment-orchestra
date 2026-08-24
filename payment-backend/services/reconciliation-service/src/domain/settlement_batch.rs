//! SettlementBatch aggregate root. Event-sourced (AGG-01).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::events::ReconciliationEvent;
use super::types::BatchStatus;

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
    /// Events pending persistence to the event store.
    #[serde(default)]
    pub pending_events: Vec<ReconciliationEvent>,
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

use super::match_result::SettlementMatchOutcome;

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
            pending_events: Vec::new(),
        }
    }

    /// Apply a reconciliation event to evolve the aggregate state.
    pub fn apply_event(&mut self, event: &ReconciliationEvent) {
        self.pending_events.push(event.clone());
        match event {
            ReconciliationEvent::SettlementBatchIngested(e) => {
                self.settlement_batch_id = e.settlement_batch_id;
                self.operator_id = e.operator_id;
                self.acquirer_link_id = e.acquirer_link_id;
                self.total_records = e.total_records;
                self.total_amount_minor = e.total_amount_minor;
                self.ingested_at = e.occurred_at;
            }
            ReconciliationEvent::SettlementRecordMatched(_e) => {
                self.matched_count += 1;
            }
            ReconciliationEvent::SettlementRecordUnmatched(_e) => {
                self.unmatched_count += 1;
            }
            ReconciliationEvent::SettlementCompleted(e) => {
                self.status = BatchStatus::Processed;
                self.processed_at = Some(e.occurred_at);
            }
            ReconciliationEvent::SettlementOverdue(e) => {
                self.status = BatchStatus::Quarantined;
                self.processed_at = Some(e.occurred_at);
            }
            // Other events don't affect SettlementBatch state directly
            _ => {}
        }
    }
}

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{SettlementBatchStatus, SettlementMatchOutcome, SettlementRecord};
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct SettlementBatch {
    pub settlement_batch_id: Uuid,
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub file_checksum: String,
    pub file_format: String,
    pub status: SettlementBatchStatus,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount_minor_units: i64,
    pub ingested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

impl SettlementBatch {
    pub fn new(
        operator_id: Uuid,
        acquirer_link_id: Uuid,
        file_checksum: String,
        file_format: String,
    ) -> Self {
        Self {
            settlement_batch_id: Uuid::now_v7(),
            operator_id,
            acquirer_link_id,
            file_checksum,
            file_format,
            status: SettlementBatchStatus::Ingesting,
            total_records: 0,
            matched_count: 0,
            unmatched_count: 0,
            total_amount_minor_units: 0,
            ingested_at: Utc::now(),
            processed_at: None,
        }
    }

    pub fn record_match(&mut self) {
        self.matched_count += 1;
    }

    pub fn record_unmatch(&mut self) {
        self.unmatched_count += 1;
    }

    pub fn complete(&mut self) {
        self.status = SettlementBatchStatus::Processed;
        self.processed_at = Some(Utc::now());
    }
}

#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub debit_amount_minor_units: i64,
    pub credit_amount_minor_units: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
}

impl LedgerEntry {
    pub fn new(
        transaction_id: Uuid,
        entry_type: String,
        debit: i64,
        credit: i64,
        currency: String,
        source_acquirer: String,
    ) -> Self {
        Self {
            entry_id: Uuid::now_v7(),
            transaction_id,
            entry_type,
            debit_amount_minor_units: debit,
            credit_amount_minor_units: credit,
            currency,
            source_acquirer,
            reconciliation_batch_id: None,
            reconciled: false,
            created_at: Utc::now(),
        }
    }
}

pub struct ReconciliationMatcher {
    pub auto_confirm_threshold: f64,
    pub review_threshold: f64,
}

impl ReconciliationMatcher {
    pub fn new() -> Self {
        Self {
            auto_confirm_threshold: 0.95,
            review_threshold: 0.70,
        }
    }

    pub fn match_record(
        &self,
        record: &SettlementRecord,
        acquirer_references: &[(Uuid, String)], // (payment_intent_id, acquirer_reference)
    ) -> crate::domain::value_objects::MatchResult {
        // Exact match by acquirer reference
        for (intent_id, reference) in acquirer_references {
            if reference == &record.acquirer_reference {
                return crate::domain::value_objects::MatchResult {
                    payment_intent_id: Some(*intent_id),
                    confidence: 1.0,
                    outcome: SettlementMatchOutcome::Matched,
                    auto_confirm: true,
                };
            }
        }

        // No match found
        crate::domain::value_objects::MatchResult {
            payment_intent_id: None,
            confidence: 0.0,
            outcome: SettlementMatchOutcome::Unmatched,
            auto_confirm: false,
        }
    }
}

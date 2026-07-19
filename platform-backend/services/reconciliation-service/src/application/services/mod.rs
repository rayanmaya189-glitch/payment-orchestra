use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{LedgerEntry, SettlementBatch};
use crate::domain::value_objects::SettlementRecord;
use crate::infrastructure::repository::{LedgerRepository, SettlementBatchRepository};
use platform_error::PlatformError;

#[async_trait]
pub trait ReconciliationService: Send + Sync {
    async fn ingest_settlement_batch(&self, cmd: IngestSettlementBatchCommand) -> Result<SettlementBatchResponse, PlatformError>;
    async fn get_batch(&self, batch_id: Uuid) -> Result<SettlementBatchResponse, PlatformError>;
    async fn verify_ledger_balance(&self, transaction_id: Uuid) -> Result<bool, PlatformError>;
}

pub struct ReconciliationServiceImpl {
    batch_repo: Box<dyn SettlementBatchRepository>,
    ledger_repo: Box<dyn LedgerRepository>,
}

impl ReconciliationServiceImpl {
    pub fn new(
        batch_repo: Box<dyn SettlementBatchRepository>,
        ledger_repo: Box<dyn LedgerRepository>,
    ) -> Self {
        Self { batch_repo, ledger_repo }
    }
}

#[derive(Debug, Clone)]
pub struct IngestSettlementBatchCommand {
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub raw_file: Vec<u8>,
    pub file_format: String,
}

#[derive(Debug, Clone)]
pub struct SettlementBatchResponse {
    pub settlement_batch_id: Uuid,
    pub status: String,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount: i64,
}

#[async_trait]
impl ReconciliationService for ReconciliationServiceImpl {
    async fn ingest_settlement_batch(&self, cmd: IngestSettlementBatchCommand) -> Result<SettlementBatchResponse, PlatformError> {
        use sha2::{Sha256, Digest};

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(&cmd.raw_file);
        let checksum = format!("{:x}", hasher.finalize());

        // Check for duplicate (INV-006)
        if self.batch_repo.find_by_checksum(&checksum).await?.is_some() {
            return Err(PlatformError::Conflict(
                platform_error::ConflictError::IdempotencyKeyConflict
            ));
        }

        // Create batch
        let mut batch = SettlementBatch::new(
            cmd.operator_id,
            cmd.acquirer_link_id,
            checksum,
            cmd.file_format,
        );

        // Parse records (simplified - in real impl would parse CSV/webhook)
        let records: Vec<SettlementRecord> = Vec::new(); // TODO: Parse from raw_file

        batch.total_records = records.len() as i32;
        batch.total_amount_minor_units = records.iter().map(|r| r.amount_minor_units).sum();

        // Match records
        let matcher = crate::domain::aggregates::ReconciliationMatcher::new();
        let acquirer_references: Vec<(Uuid, String)> = Vec::new(); // TODO: Load from payment_intents

        for record in &records {
            let result = matcher.match_record(record, &acquirer_references);
            match result.outcome {
                crate::domain::value_objects::SettlementMatchOutcome::Matched => {
                    batch.record_match();

                    // Create ledger entries
                    let debit_entry = LedgerEntry::new(
                        result.payment_intent_id.unwrap_or_default(),
                        "settlement_credit".to_string(),
                        0,
                        record.amount_minor_units,
                        record.currency.clone(),
                        "acquirer".to_string(),
                    );
                    self.ledger_repo.save(&debit_entry).await?;
                }
                _ => {
                    batch.record_unmatch();
                }
            }
        }

        batch.complete();
        self.batch_repo.save(&batch).await?;

        Ok(SettlementBatchResponse {
            settlement_batch_id: batch.settlement_batch_id,
            status: batch.status.as_str().to_string(),
            total_records: batch.total_records,
            matched_count: batch.matched_count,
            unmatched_count: batch.unmatched_count,
            total_amount: batch.total_amount_minor_units,
        })
    }

    async fn get_batch(&self, batch_id: Uuid) -> Result<SettlementBatchResponse, PlatformError> {
        let batch = self.batch_repo
            .load(batch_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "SettlementBatch".into(),
                id: batch_id,
            })?;

        Ok(SettlementBatchResponse {
            settlement_batch_id: batch.settlement_batch_id,
            status: batch.status.as_str().to_string(),
            total_records: batch.total_records,
            matched_count: batch.matched_count,
            unmatched_count: batch.unmatched_count,
            total_amount: batch.total_amount_minor_units,
        })
    }

    async fn verify_ledger_balance(&self, transaction_id: Uuid) -> Result<bool, PlatformError> {
        self.ledger_repo.verify_balance(transaction_id).await
    }
}

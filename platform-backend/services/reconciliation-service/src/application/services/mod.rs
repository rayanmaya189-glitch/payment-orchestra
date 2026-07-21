use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::{BatchMatchStats, SettlementBatch, SettlementMatcher};
use crate::domain::rules::*;
use crate::domain::value_objects::SettlementStatus;
use platform_error::PlatformError;

// ==================== Service Trait ====================

#[async_trait]
pub trait ReconciliationService: Send + Sync {
    /// Poll and create a new settlement batch.
    async fn poll(&self, cmd: PollSettlementCommand) -> Result<Uuid, PlatformError>;

    /// Ingest a settlement file: parse, dedup-check, store, and match.
    async fn ingest(&self, cmd: IngestSettlementCommand) -> Result<Uuid, PlatformError>;

    /// Run matching on an already-polled batch.
    async fn match_batch(
        &self,
        cmd: MatchSettlementCommand,
    ) -> Result<MatchResultResponse, PlatformError>;

    /// Get a batch by ID.
    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError>;

    /// List batches for an operator.
    async fn list_batches(
        &self,
        cmd: ListBatchesCommand,
    ) -> Result<Vec<SettlementBatch>, PlatformError>;

    /// Finalize a matched batch (mark as settled).
    async fn finalize_batch(&self, batch_id: Uuid) -> Result<(), PlatformError>;
}

#[derive(Debug, Clone)]
pub struct MatchResultResponse {
    pub batch_id: Uuid,
    pub matched: i32,
    pub unmatched: i32,
    pub exceptions: Vec<String>,
    pub stats: BatchMatchStats,
}

// ==================== Service Implementation ====================

pub struct ReconciliationServiceImpl {
    batch_repo: Box<dyn SettlementBatchRepository>,
    record_repo: Box<dyn SettlementRecordRepository>,
    ledger_repo: Box<dyn LedgerRepository>,
    idempotency: Box<dyn BatchIdempotencyGuard>,
    db: DatabaseConnection,
}

impl ReconciliationServiceImpl {
    pub fn new(
        batch_repo: Box<dyn SettlementBatchRepository>,
        record_repo: Box<dyn SettlementRecordRepository>,
        ledger_repo: Box<dyn LedgerRepository>,
        idempotency: Box<dyn BatchIdempotencyGuard>,
        db: DatabaseConnection,
    ) -> Self {
        Self { batch_repo, record_repo, ledger_repo, idempotency, db }
    }
}

#[async_trait]
impl ReconciliationService for ReconciliationServiceImpl {
    async fn poll(&self, cmd: PollSettlementCommand) -> Result<Uuid, PlatformError> {
        let mut batch = SettlementBatch::new(
            cmd.operator_id,
            cmd.connector_id,
            cmd.period_start,
            cmd.period_end,
        );
        batch.mark_polled(0, 0);
        self.batch_repo.save(&batch).await?;

        tracing::info!(
            batch_id = %batch.batch_id,
            connector_id = %batch.connector_id,
            "Settlement batch created (polled)"
        );

        Ok(batch.batch_id)
    }

    async fn ingest(&self, cmd: IngestSettlementCommand) -> Result<Uuid, PlatformError> {
        // INV-06: Check for duplicate batch by checksum
        if self.idempotency.is_duplicate(&cmd.file_checksum).await? {
            return Err(PlatformError::Conflict(
                platform_error::ConflictError::IdempotencyKeyConflict,
            ));
        }

        // Parse the settlement file
        let format = match cmd.file_format.as_str() {
            "csv" => crate::infrastructure::adapters::settlement_parser::SettlementFileFormat::Csv,
            "json" => crate::infrastructure::adapters::settlement_parser::SettlementFileFormat::Json,
            "fixed_width" => {
                crate::infrastructure::adapters::settlement_parser::SettlementFileFormat::FixedWidth
            }
            _ => {
                return Err(PlatformError::Validation(
                    platform_error::ValidationError::MissingField(format!(
                        "Unsupported file format: {}",
                        cmd.file_format
                    )),
                ));
            }
        };

        let records = crate::infrastructure::adapters::settlement_parser::parse_settlement_file(
            &cmd.raw_content,
            format,
            &cmd.connector_id,
        )?;

        // Calculate total amount
        let total_amount: i64 = records.iter().map(|r| r.amount.amount_minor_units).sum();

        // Create batch
        let mut batch = SettlementBatch::new(
            cmd.operator_id,
            cmd.connector_id.clone(),
            String::new(),
            String::new(),
        );
        batch.mark_polled(records.len() as i32, total_amount);

        // Store parsed records
        self.record_repo.save_records(batch.batch_id, &records).await?;

        // Load internal ledger entries for this connector/period and run matching
        let ledger_entries = self.ledger_repo.find_in_period("", "", &cmd.connector_id).await?;
        let results = SettlementMatcher::match_batch(&records, &ledger_entries);
        let stats = SettlementMatcher::compute_stats(&results);

        // Update batch with match results
        batch.mark_matched(stats.matched, stats.unmatched + stats.amount_mismatch + stats.duplicate_reference + stats.fee_discrepancy);
        batch.exception_count = stats.amount_mismatch + stats.duplicate_reference + stats.fee_discrepancy;

        if !stats.exceptions.is_empty() {
            batch.mark_exception(serde_json::json!(stats.exceptions));
        }

        self.batch_repo.save(&batch).await?;
        self.record_repo.save_match_results(batch.batch_id, &results).await?;

        // Record checksum for idempotency
        self.idempotency.record_checksum(batch.batch_id, &cmd.file_checksum).await?;

        tracing::info!(
            batch_id = %batch.batch_id,
            total = stats.total,
            matched = stats.matched,
            unmatched = stats.unmatched,
            exceptions = stats.exceptions.len(),
            "Settlement batch ingested and matched"
        );

        Ok(batch.batch_id)
    }

    async fn match_batch(
        &self,
        cmd: MatchSettlementCommand,
    ) -> Result<MatchResultResponse, PlatformError> {
        let mut batch = self
            .batch_repo
            .find_by_id(cmd.batch_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "settlement_batch".into(),
                id: cmd.batch_id,
            })?;

        if batch.status != SettlementStatus::Polled {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: batch.status.as_str().to_string(),
                    command: "MatchSettlement".into(),
                },
            ));
        }

        // Load settlement records for this batch
        let settlement_records = self.record_repo.load_records(cmd.batch_id).await?;

        // Load ledger entries
        let ledger_entries = self
            .ledger_repo
            .find_in_period(&batch.period_start, &batch.period_end, &batch.connector_id)
            .await?;

        // Run matching algorithm
        let match_results = SettlementMatcher::match_batch(&settlement_records, &ledger_entries);
        let stats = SettlementMatcher::compute_stats(&match_results);

        // Update batch
        batch.mark_matched(stats.matched, stats.unmatched);

        if !stats.exceptions.is_empty() {
            batch.mark_exception(serde_json::json!(stats.exceptions));
        }

        self.batch_repo.save(&batch).await?;
        self.record_repo.save_match_results(cmd.batch_id, &match_results).await?;

        tracing::info!(
            batch_id = %cmd.batch_id,
            matched = stats.matched,
            unmatched = stats.unmatched,
            "Batch matching completed"
        );

        let exceptions = stats.exceptions.clone();
        Ok(MatchResultResponse {
            batch_id: batch.batch_id,
            matched: stats.matched,
            unmatched: stats.unmatched,
            exceptions,
            stats,
        })
    }

    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError> {
        self.batch_repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound {
            resource: "settlement_batch".into(),
            id,
        })
    }

    async fn list_batches(
        &self,
        cmd: ListBatchesCommand,
    ) -> Result<Vec<SettlementBatch>, PlatformError> {
        self.batch_repo
            .find_by_operator(cmd.operator_id, cmd.limit, cmd.offset)
            .await
    }

    async fn finalize_batch(&self, batch_id: Uuid) -> Result<(), PlatformError> {
        let mut batch = self.batch_repo.find_by_id(batch_id).await?.ok_or_else(|| {
            PlatformError::NotFound { resource: "settlement_batch".into(), id: batch_id }
        })?;

        if batch.status != SettlementStatus::Matched {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: batch.status.as_str().to_string(),
                    command: "finalize".into(),
                },
            ));
        }

        batch.mark_settled();
        self.batch_repo.save(&batch).await?;

        tracing::info!(batch_id = %batch_id, "Batch finalized (settled)");
        Ok(())
    }
}

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::SettlementBatch;
use crate::domain::rules::SettlementBatchRepository;
use platform_error::PlatformError;

pub struct ReconciliationServiceImpl { repo: Box<dyn SettlementBatchRepository>, db: DatabaseConnection }
impl ReconciliationServiceImpl { pub fn new(repo: Box<dyn SettlementBatchRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait ReconciliationService: Send + Sync {
    async fn poll(&self, cmd: PollSettlementCommand) -> Result<Uuid, PlatformError>;
    async fn match_batch(&self, cmd: MatchSettlementCommand) -> Result<MatchResult, PlatformError>;
    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError>;
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub batch_id: Uuid,
    pub matched: i32,
    pub unmatched: i32,
    pub exceptions: Vec<String>,
}

#[async_trait]
impl ReconciliationService for ReconciliationServiceImpl {
    async fn poll(&self, cmd: PollSettlementCommand) -> Result<Uuid, PlatformError> {
        let mut batch = SettlementBatch::new(cmd.operator_id, cmd.connector_id, cmd.period_start, cmd.period_end);
        batch.mark_polled(0, 0);
        self.repo.save(&batch).await?;
        Ok(batch.batch_id)
    }

    async fn match_batch(&self, cmd: MatchSettlementCommand) -> Result<MatchResult, PlatformError> {
        let mut batch = self.repo.find_by_id(cmd.batch_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "settlement_batch".into(), id: cmd.batch_id })?;

        if batch.status != crate::domain::value_objects::SettlementStatus::Polled {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: batch.status.as_str().to_string(),
                    command: "MatchSettlement".into(),
                }
            ));
        }

        // Settlement matching logic (SRS Part 3 §3.2)
        // In production, this would:
        // 1. Load internal ledger entries for the period
        // 2. Load connector settlement records (polled earlier)
        // 3. Match by acquirer_reference (exact match)
        // 4. Flag unmatched records as exceptions
        // For now, we accept pre-computed counts from the polling stage
        let matched = cmd.matched_count;
        let unmatched = cmd.unmatched_count;
        let exceptions: Vec<String> = cmd.exceptions.unwrap_or_default();

        batch.mark_matched(matched, unmatched);
        if !exceptions.is_empty() {
            batch.exceptions = Some(serde_json::json!(exceptions));
        }
        self.repo.save(&batch).await?;

        Ok(MatchResult {
            batch_id: batch.batch_id,
            matched,
            unmatched,
            exceptions,
        })
    }

    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "settlement_batch".into(), id })
    }
}

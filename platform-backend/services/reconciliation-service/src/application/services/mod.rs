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
    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError>;
}

#[async_trait]
impl ReconciliationService for ReconciliationServiceImpl {
    async fn poll(&self, cmd: PollSettlementCommand) -> Result<Uuid, PlatformError> {
        let mut batch = SettlementBatch::new(cmd.operator_id, cmd.connector_id, cmd.period_start, cmd.period_end);
        batch.mark_polled(0, 0);
        self.repo.save(&batch).await?;
        Ok(batch.batch_id)
    }
    async fn get_batch(&self, id: Uuid) -> Result<SettlementBatch, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "settlement_batch".into(), id })
    }
}

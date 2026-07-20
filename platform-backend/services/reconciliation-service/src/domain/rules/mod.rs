use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::SettlementBatch;
use platform_error::PlatformError;
#[async_trait]
pub trait SettlementBatchRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SettlementBatch>, PlatformError>;
    async fn save(&self, batch: &SettlementBatch) -> Result<(), PlatformError>;
}

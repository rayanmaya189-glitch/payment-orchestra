use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{LedgerEntry, SettlementBatch};
use platform_error::PlatformError;

#[async_trait]
pub trait SettlementBatchRepository: Send + Sync {
    async fn save(&self, batch: &SettlementBatch) -> Result<(), PlatformError>;
    async fn load(&self, id: Uuid) -> Result<Option<SettlementBatch>, PlatformError>;
    async fn find_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, PlatformError>;
}

#[async_trait]
pub trait LedgerRepository: Send + Sync {
    async fn save(&self, entry: &LedgerEntry) -> Result<(), PlatformError>;
    async fn find_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<LedgerEntry>, PlatformError>;
    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, PlatformError>;
}

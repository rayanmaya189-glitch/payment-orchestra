//! Outbox Relay repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait OutboxRepository: Send + Sync {
    async fn load_entry(&self, outbox_id: Uuid) -> Result<Option<OutboxEntry>, OutboxRelayError>;
    async fn save_entry(&self, entry: &OutboxEntry) -> Result<(), OutboxRelayError>;
    async fn find_unpublished(&self, batch_size: u32) -> Result<Vec<OutboxEntry>, OutboxRelayError>;
    async fn mark_published(&self, outbox_id: Uuid) -> Result<(), OutboxRelayError>;
    async fn count_unpublished(&self) -> Result<u64, OutboxRelayError>;
    async fn list_entries(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError>;
}

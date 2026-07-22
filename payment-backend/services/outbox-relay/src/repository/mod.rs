//! Outbox Relay repository

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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

#[derive(Clone)]
pub struct InMemoryOutboxRepository {
    entries: Arc<RwLock<HashMap<Uuid, OutboxEntry>>>,
}

impl InMemoryOutboxRepository {
    pub fn new() -> Self {
        Self { entries: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl OutboxRepository for InMemoryOutboxRepository {
    async fn load_entry(&self, outbox_id: Uuid) -> Result<Option<OutboxEntry>, OutboxRelayError> {
        Ok(self.entries.read().await.get(&outbox_id).cloned())
    }

    async fn save_entry(&self, entry: &OutboxEntry) -> Result<(), OutboxRelayError> {
        self.entries.write().await.insert(entry.outbox_id, entry.clone());
        Ok(())
    }

    async fn find_unpublished(&self, batch_size: u32) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.published_at.is_none())
            .take(batch_size as usize)
            .cloned()
            .collect())
    }

    async fn mark_published(&self, outbox_id: Uuid) -> Result<(), OutboxRelayError> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&outbox_id) {
            entry.published_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn count_unpublished(&self) -> Result<u64, OutboxRelayError> {
        let entries = self.entries.read().await;
        Ok(entries.values().filter(|e| e.published_at.is_none()).count() as u64)
    }

    async fn list_entries(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        Ok(self.entries.read().await.values().cloned().collect())
    }
}

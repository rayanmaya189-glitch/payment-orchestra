//! Outbox Relay pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct OutboxRelayPipeline {
    pub api: OutboxRelayApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryOutboxRepository>>,
}

use platform_messaging::event_bus::{EventBus, NoopEventBus};

impl OutboxRelayPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryOutboxRepository::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let config = OutboxRelayConfig::default();
        let metrics = Arc::new(RwLock::new(RelayMetrics {
            total_polled: 0, total_published: 0, total_failed: 0,
            total_duplicates_skipped: 0, last_polled_at: None,
            queue_depth: 0, is_running: false,
        }));
        let ch: Box<dyn CommandHandler> = Box::new(OutboxCommandHandler::new(adapter.clone(), config.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(OutboxQueryHandler::new(adapter, config, metrics));
        let api = OutboxRelayApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self { api, event_bus: eb, repo }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryOutboxRepository>>);

#[async_trait::async_trait]
impl OutboxRepository for ArcRepoAdapter {
    async fn load_entry(&self, id: uuid::Uuid) -> Result<Option<OutboxEntry>, OutboxRelayError> {
        self.0.read().await.load_entry(id).await
    }
    async fn save_entry(&self, entry: &OutboxEntry) -> Result<(), OutboxRelayError> {
        self.0.write().await.save_entry(entry).await
    }
    async fn find_unpublished(&self, batch: u32) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        self.0.read().await.find_unpublished(batch).await
    }
    async fn mark_published(&self, id: uuid::Uuid) -> Result<(), OutboxRelayError> {
        self.0.write().await.mark_published(id).await
    }
    async fn count_unpublished(&self) -> Result<u64, OutboxRelayError> {
        self.0.read().await.count_unpublished().await
    }
    async fn list_entries(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        self.0.read().await.list_entries().await
    }
}

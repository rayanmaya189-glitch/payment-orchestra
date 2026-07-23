//! Analytics Service dependency injection pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

pub struct AnalyticsPipeline {
    pub api: AnalyticsApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryAnalyticsStore>>,
}

impl AnalyticsPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryAnalyticsStore::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let ch: Box<dyn CommandHandler> = Box::new(AnalyticsCommandHandler::new(adapter.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(AnalyticsQueryHandler::new(adapter));
        let api = AnalyticsApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self {
            api,
            event_bus: eb,
            repo,
        }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryAnalyticsStore>>);

#[async_trait::async_trait]
impl AnalyticsRepository for ArcRepoAdapter {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError> {
        self.0.write().await.store_event(event).await
    }
    async fn get_events_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        self.0.read().await.get_events_in_range(start, end).await
    }
    async fn get_events_by_type(
        &self,
        event_type: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        self.0.read().await.get_events_by_type(event_type, start, end).await
    }
    async fn get_events_by_acquirer(
        &self,
        acquirer_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        self.0
            .read()
            .await
            .get_events_by_acquirer(acquirer_id, start, end)
            .await
    }
    async fn last_ingested_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.0.read().await.last_ingested_at().await
    }
    async fn event_count(&self) -> u64 {
        self.0.read().await.event_count().await
    }
}

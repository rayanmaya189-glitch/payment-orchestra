//! Saga Coordinator command processing pipeline — BC-17

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

pub struct SagaPipeline {
    pub api: SagaApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemorySagaRepository>>,
}

impl SagaPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemorySagaRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(SagaCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(SagaQueryHandler::new(repo_adapter));
        let api = SagaApi::new(command_handler, query_handler);
        let event_bus: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus,
            repo,
        }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemorySagaRepository>>);

#[async_trait::async_trait]
impl SagaRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<SagaInstance>, SagaError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn save(&self, saga: &SagaInstance) -> Result<(), SagaError> {
        let repo = self.0.write().await;
        repo.save(saga).await
    }

    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError> {
        let repo = self.0.read().await;
        repo.find_stuck(timeout_seconds).await
    }

    async fn find_by_aggregate(&self, aggregate_id: uuid::Uuid) -> Result<Vec<SagaInstance>, SagaError> {
        let repo = self.0.read().await;
        repo.find_by_aggregate(aggregate_id).await
    }
}

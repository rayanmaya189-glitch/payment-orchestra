//! Dispute Management command processing pipeline — BC-10

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Aggregates all dependencies for the dispute service.
pub struct DisputePipeline {
    pub api: DisputeApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryDisputeRepository>>,
}

impl DisputePipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryDisputeRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(DisputeCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(DisputeQueryHandler::new(repo_adapter));
        let api = DisputeApi::new(command_handler, query_handler);
        let event_bus: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus,
            repo,
        }
    }
}

/// Adapter that wraps `Arc<RwLock<InMemoryDisputeRepository>>` and
/// implements `DisputeRepository` by delegating to the inner repo.
#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryDisputeRepository>>);

#[async_trait::async_trait]
impl DisputeRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<ChargebackCase>, DisputeError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn save(&self, case: &ChargebackCase) -> Result<(), DisputeError> {
        let repo = self.0.write().await;
        repo.save(case).await
    }

    async fn find_by_payment_intent(
        &self,
        payment_intent_id: uuid::Uuid,
    ) -> Result<Vec<ChargebackCase>, DisputeError> {
        let repo = self.0.read().await;
        repo.find_by_payment_intent(payment_intent_id).await
    }

    async fn find_open_cases(&self, operator_id: uuid::Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let repo = self.0.read().await;
        repo.find_open_cases(operator_id).await
    }

    async fn find_by_operator(&self, operator_id: uuid::Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        let repo = self.0.read().await;
        repo.find_by_operator(operator_id).await
    }
}

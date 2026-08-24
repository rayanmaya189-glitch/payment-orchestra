//! Subscription Billing command processing pipeline — BC-08

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

/// Aggregates all dependencies for the subscription service.
pub struct SubscriptionPipeline {
    pub api: SubscriptionApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemorySubscriptionRepository>>,
}

impl Default for SubscriptionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemorySubscriptionRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(SubscriptionCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(SubscriptionQueryHandler::new(repo_adapter));
        let api = SubscriptionApi::new(command_handler, query_handler);
        let event_bus: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus,
            repo,
        }
    }

    /// Override the default no-op event bus with a real NATS-backed bus.
    pub fn with_event_bus(mut self, event_bus: Box<dyn EventBus>) -> Self {
        self.event_bus = event_bus;
        self
    }
}

/// Adapter that wraps `Arc<RwLock<InMemorySubscriptionRepository>>` and
/// implements `SubscriptionRepository` by delegating to the inner repo.
#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemorySubscriptionRepository>>);

#[async_trait::async_trait]
impl SubscriptionRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<Subscription>, SubscriptionError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn save(&self, subscription: &mut Subscription) -> Result<(), SubscriptionError> {
        let repo = self.0.write().await;
        repo.save(subscription).await
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        let repo = self.0.read().await;
        repo.find_active_for_renewal().await
    }

    async fn find_by_customer(&self, customer_id: uuid::Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let repo = self.0.read().await;
        repo.find_by_customer(customer_id).await
    }

    async fn find_by_operator(&self, operator_id: uuid::Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let repo = self.0.read().await;
        repo.find_by_operator(operator_id).await
    }
}

//! Payment Link command processing pipeline — BC-07

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

/// Aggregates all dependencies for the payment-link service.
pub struct PaymentLinkPipeline {
    pub api: PaymentLinkApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryPaymentLinkRepository>>,
}

impl PaymentLinkPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryPaymentLinkRepository::new()));
        let repo_clone = repo.clone();

        let repo_adapter = ArcRepoAdapter(repo_clone);

        let command_handler: Box<dyn CommandHandler> =
            Box::new(PaymentLinkCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(PaymentLinkQueryHandler::new(repo_adapter));
        let api = PaymentLinkApi::new(command_handler, query_handler);
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

/// Adapter that wraps `Arc<RwLock<InMemoryPaymentLinkRepository>>` and
/// implements `PaymentLinkRepository` by delegating to the inner repo.
#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryPaymentLinkRepository>>);

#[async_trait::async_trait]
impl PaymentLinkRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn load_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let repo = self.0.read().await;
        repo.load_by_token(token).await
    }

    async fn save(&self, link: &PaymentLink) -> Result<(), PaymentLinkError> {
        let repo = self.0.write().await;
        repo.save(link).await
    }

    async fn find_by_operator(&self, operator_id: uuid::Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let repo = self.0.read().await;
        repo.find_by_operator(operator_id).await
    }

    async fn find_expired(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let repo = self.0.read().await;
        repo.find_expired().await
    }
}

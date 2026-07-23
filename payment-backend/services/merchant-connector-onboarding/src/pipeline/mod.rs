use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

pub struct OnboardingPipeline {
    pub api: OnboardingApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryOnboardingRepository>>,
}

impl OnboardingPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryOnboardingRepository::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let ch: Box<dyn CommandHandler> = Box::new(OnboardingCommandHandler::new(adapter.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(OnboardingQueryHandler::new(adapter));
        let api = OnboardingApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self { api, event_bus: eb, repo }
    }

    /// Override the default no-op event bus with a real NATS-backed bus.
    pub fn with_event_bus(mut self, event_bus: Box<dyn EventBus>) -> Self {
        self.event_bus = event_bus;
        self
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryOnboardingRepository>>);

#[async_trait::async_trait]
impl OnboardingRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<OnboardingRequest>, OnboardingError> {
        self.0.read().await.load(id).await
    }
    async fn save(&self, r: &OnboardingRequest) -> Result<(), OnboardingError> {
        self.0.write().await.save(r).await
    }
    async fn find_by_operator(&self, oid: uuid::Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        self.0.read().await.find_by_operator(oid).await
    }
    async fn find_active(&self, oid: uuid::Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        self.0.read().await.find_active(oid).await
    }
}

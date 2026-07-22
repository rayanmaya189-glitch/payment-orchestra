use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: OnboardingEvent) -> Result<(), OnboardingError>;
}

pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _event: OnboardingEvent) -> Result<(), OnboardingError> { Ok(()) }
}

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

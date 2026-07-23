//! AI Assistant Service dependency injection pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AiAssistantPipeline {
    pub api: AiAssistantApi,
    pub event_bus: Box<dyn EventBus>,
    pub session_repo: Arc<RwLock<InMemoryConversationSessionRepository>>,
}

use platform_messaging::event_bus::{EventBus, NoopEventBus};

impl AiAssistantPipeline {
    pub fn new() -> Self {
        let session_repo = Arc::new(RwLock::new(InMemoryConversationSessionRepository::new()));
        let adapter = ArcRepoAdapter(session_repo.clone());

        let rag_engine: Box<dyn RagEngine> = Box::new(SimulatedRagEngine);
        let rate_limiter: Box<dyn RateLimiter> = Box::new(NoopRateLimiter);

        let ch: Box<dyn CommandHandler> = Box::new(AiCommandHandler::new(
            adapter.clone(),
            rag_engine,
            rate_limiter,
        ));
        let qh: Box<dyn QueryHandler> = Box::new(AiQueryHandler::new(adapter));
        let api = AiAssistantApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus: eb,
            session_repo,
        }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryConversationSessionRepository>>);

#[async_trait::async_trait]
impl ConversationSessionRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<ConversationSession>, AiError> {
        self.0.read().await.load(id).await
    }
    async fn save(&self, session: &ConversationSession) -> Result<(), AiError> {
        self.0.write().await.save(session).await
    }
    async fn find_by_operator(&self, oid: uuid::Uuid) -> Result<Vec<ConversationSession>, AiError> {
        self.0.read().await.find_by_operator(oid).await
    }
    async fn delete(&self, id: uuid::Uuid) -> Result<(), AiError> {
        self.0.write().await.delete(id).await
    }
}

//! AI Assistant Service commands

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

pub mod conversation;
pub mod rag;
pub mod rate_limiter;
pub use rag::SimulatedRagEngine;
pub use rate_limiter::NoopRateLimiter;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn start_conversation(&self, cmd: StartConversation) -> Result<ConversationSession, AiError>;
    async fn ask_question(&self, cmd: AskQuestion) -> Result<RagResult, AiError>;
}

pub struct AiCommandHandler<R: ConversationSessionRepository> {
    pub(super) session_repo: R,
    pub(super) rag_engine: Box<dyn RagEngine>,
    pub(super) rate_limiter: Box<dyn RateLimiter>,
}

impl<R: ConversationSessionRepository> AiCommandHandler<R> {
    pub fn new(
        session_repo: R,
        rag_engine: Box<dyn RagEngine>,
        rate_limiter: Box<dyn RateLimiter>,
    ) -> Self {
        Self {
            session_repo,
            rag_engine,
            rate_limiter,
        }
    }
}

#[async_trait]
pub trait RagEngine: Send + Sync {
    async fn answer_question(
        &self,
        question: &str,
        classification: QueryClassification,
        context: &ConversationSession,
    ) -> Result<RagResult, AiError>;
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self, operator_id: Uuid) -> Result<(), AiError>;
}

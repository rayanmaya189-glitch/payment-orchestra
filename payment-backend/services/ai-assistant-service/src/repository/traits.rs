//! ConversationSessionRepository trait for AI Assistant Service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait ConversationSessionRepository: Send + Sync {
    async fn load(&self, session_id: Uuid) -> Result<Option<ConversationSession>, AiError>;
    async fn save(&self, session: &ConversationSession) -> Result<(), AiError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError>;
    async fn delete(&self, session_id: Uuid) -> Result<(), AiError>;
}

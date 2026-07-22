//! AI Assistant Service repository

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

// ---------------------------------------------------------------------------
// ConversationSessionRepository trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ConversationSessionRepository: Send + Sync {
    async fn load(&self, session_id: Uuid) -> Result<Option<ConversationSession>, AiError>;
    async fn save(&self, session: &ConversationSession) -> Result<(), AiError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError>;
    async fn delete(&self, session_id: Uuid) -> Result<(), AiError>;
}

// ---------------------------------------------------------------------------
// InMemoryConversationSessionRepository
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InMemoryConversationSessionRepository {
    sessions: Arc<RwLock<HashMap<Uuid, ConversationSession>>>,
}

impl InMemoryConversationSessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConversationSessionRepository for InMemoryConversationSessionRepository {
    async fn load(&self, session_id: Uuid) -> Result<Option<ConversationSession>, AiError> {
        let map = self.sessions.read().await;
        Ok(map.get(&session_id).cloned())
    }

    async fn save(&self, session: &ConversationSession) -> Result<(), AiError> {
        let mut map = self.sessions.write().await;
        map.insert(session.session_id, session.clone());
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError> {
        let map = self.sessions.read().await;
        Ok(map
            .values()
            .filter(|s| s.operator_id == operator_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, session_id: Uuid) -> Result<(), AiError> {
        let mut map = self.sessions.write().await;
        map.remove(&session_id);
        Ok(())
    }
}

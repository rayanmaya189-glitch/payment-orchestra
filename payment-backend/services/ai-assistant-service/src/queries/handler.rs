//! AI Assistant Service query handlers

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::queries::types::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    /// Get a conversation session by ID.
    async fn get_session(&self, session_id: Uuid, operator_id: Uuid) -> Result<ConversationSession, AiError>;
    /// List all sessions for an operator.
    async fn list_sessions(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError>;
    /// Search message history within a session.
    async fn search_session_history(
        &self,
        session_id: Uuid,
        operator_id: Uuid,
        query: &str,
    ) -> Result<Vec<ConversationMessage>, AiError>;
    /// Get session info with message count.
    async fn get_session_info(&self, session_id: Uuid) -> Result<SessionInfo, AiError>;
    /// Check AI assistant health / quota status.
    async fn health_status(&self) -> AiHealthStatus;
}

pub struct AiQueryHandler<R: ConversationSessionRepository> {
    session_repo: R,
    startup_time: std::time::Instant,
}

impl<R: ConversationSessionRepository> AiQueryHandler<R> {
    pub fn new(session_repo: R) -> Self {
        Self {
            session_repo,
            startup_time: std::time::Instant::now(),
        }
    }
}

#[async_trait]
impl<R: ConversationSessionRepository + Send + Sync> QueryHandler for AiQueryHandler<R> {
    async fn get_session(&self, session_id: Uuid, operator_id: Uuid) -> Result<ConversationSession, AiError> {
        let session = self
            .session_repo
            .load(session_id)
            .await?
            .ok_or(AiError::SessionNotFound(session_id))?;

        if session.operator_id != operator_id {
            return Err(AiError::OperatorMismatch);
        }

        Ok(session)
    }

    async fn list_sessions(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError> {
        self.session_repo.find_by_operator(operator_id).await
    }

    async fn search_session_history(
        &self,
        session_id: Uuid,
        operator_id: Uuid,
        query: &str,
    ) -> Result<Vec<ConversationMessage>, AiError> {
        let session = self.get_session(session_id, operator_id).await?;

        let lower = query.to_lowercase();
        let matching: Vec<ConversationMessage> = session
            .messages
            .into_iter()
            .filter(|m| m.content.to_lowercase().contains(&lower))
            .collect();

        // Enforce output volume limit (AI-EXFIL-001)
        let max_results = MAX_OUTPUT_RECORDS as usize;
        if matching.len() > max_results {
            return Err(AiError::QueryTooBroad {
                max_results: MAX_OUTPUT_RECORDS,
            });
        }

        Ok(matching)
    }

    async fn get_session_info(&self, session_id: Uuid) -> Result<SessionInfo, AiError> {
        let session = self
            .session_repo
            .load(session_id)
            .await?
            .ok_or(AiError::SessionNotFound(session_id))?;

        let last_message_at = session
            .messages
            .last()
            .map(|m| m.created_at);

        Ok(SessionInfo {
            session_id: session.session_id,
            operator_id: session.operator_id,
            title: session.title,
            message_count: session.messages.len(),
            last_message_at,
            created_at: session.created_at,
        })
    }

    async fn health_status(&self) -> AiHealthStatus {
        AiHealthStatus {
            available: true,
            message: "AI Assistant is operational. RAG pipeline ready.".into(),
            uptime_hours: self.startup_time.elapsed().as_secs() / 3600,
        }
    }
}

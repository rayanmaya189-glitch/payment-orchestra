//! AI Assistant Service public API layer

pub mod grpc;

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct AiAssistantApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl AiAssistantApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self {
            command_handler: ch,
            query_handler: qh,
        }
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    pub async fn start_conversation(&self, cmd: StartConversation) -> Result<ConversationSession, AiError> {
        self.command_handler.start_conversation(cmd).await
    }

    pub async fn ask_question(&self, cmd: AskQuestion) -> Result<RagResult, AiError> {
        self.command_handler.ask_question(cmd).await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_session(&self, session_id: Uuid, operator_id: Uuid) -> Result<ConversationSession, AiError> {
        self.query_handler.get_session(session_id, operator_id).await
    }

    pub async fn list_sessions(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError> {
        self.query_handler.list_sessions(operator_id).await
    }

    pub async fn search_session_history(
        &self,
        session_id: Uuid,
        operator_id: Uuid,
        query: &str,
    ) -> Result<Vec<ConversationMessage>, AiError> {
        self.query_handler
            .search_session_history(session_id, operator_id, query)
            .await
    }

    pub async fn get_session_info(&self, session_id: Uuid) -> Result<SessionInfo, AiError> {
        self.query_handler.get_session_info(session_id).await
    }

    pub async fn health_status(&self) -> AiHealthStatus {
        self.query_handler.health_status().await
    }
}

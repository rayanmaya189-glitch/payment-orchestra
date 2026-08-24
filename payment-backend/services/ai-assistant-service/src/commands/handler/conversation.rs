use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::{AiCommandHandler, CommandHandler};
use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
impl<R: ConversationSessionRepository + Send + Sync> CommandHandler for AiCommandHandler<R> {
    async fn start_conversation(&self, cmd: StartConversation) -> Result<ConversationSession, AiError> {
        let now = Utc::now();
        let session = ConversationSession {
            session_id: Uuid::now_v7(),
            operator_id: cmd.operator_id,
            title: cmd.title,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.session_repo.save(&session).await?;
        Ok(session)
    }

    async fn ask_question(&self, cmd: AskQuestion) -> Result<RagResult, AiError> {
        let session = self
            .session_repo
            .load(cmd.session_id)
            .await?
            .ok_or(AiError::SessionNotFound(cmd.session_id))?;

        if session.operator_id != cmd.operator_id {
            return Err(AiError::OperatorMismatch);
        }

        if cmd.question.len() > MAX_QUESTION_LENGTH {
            return Err(AiError::MessageTooLong {
                length: cmd.question.len(),
                max: MAX_QUESTION_LENGTH,
            });
        }

        self.rate_limiter
            .check_rate_limit(cmd.operator_id)
            .await?;

        let classification = super::rag::classify_query(&cmd.question);

        if classification == QueryClassification::Unanswerable {
            return Err(AiError::InsufficientGrounding);
        }

        let result = self
            .rag_engine
            .answer_question(&cmd.question, classification, &session)
            .await?;

        let mut session = session;
        session.messages.push(ConversationMessage {
            message_id: Uuid::now_v7(),
            role: MessageRole::User,
            content: cmd.question.clone(),
            citations: Vec::new(),
            created_at: Utc::now(),
        });
        session.messages.push(ConversationMessage {
            message_id: Uuid::now_v7(),
            role: MessageRole::Assistant,
            content: result.answer.clone(),
            citations: result.citations.clone(),
            created_at: Utc::now(),
        });

        if session.messages.len() > MAX_HISTORY_WINDOW {
            let excess = session.messages.len() - MAX_HISTORY_WINDOW;
            session.messages = session.messages.split_off(excess);
        }

        session.updated_at = Utc::now();
        self.session_repo.save(&session).await?;

        Ok(result)
    }
}

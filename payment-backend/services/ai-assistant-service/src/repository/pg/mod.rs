//! PostgreSQL repository for AI Assistant Service — BC-12.
//!
//! Persists [`ConversationSession`] to the `conversation_sessions` table.
//! Messages are stored as JSON in the `messages` column.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::conversation_session::{self, Entity as ConversationSessionEntity, Column as ConversationSessionColumn};
use crate::repository::ConversationSessionRepository;

#[derive(Clone)]
pub struct PostgresConversationSessionRepository {
    db: sea_orm::DatabaseConnection,
}

impl PostgresConversationSessionRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConversationSessionRepository for PostgresConversationSessionRepository {
    async fn load(&self, session_id: Uuid) -> Result<Option<ConversationSession>, AiError> {
        let result = ConversationSessionEntity::find_by_id(session_id)
            .one(&self.db)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, session: &ConversationSession) -> Result<(), AiError> {
        let model = domain_to_model(session);

        conversation_session::Entity::insert(model.clone())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(conversation_session::Column::SessionId)
                    .update_columns([
                        conversation_session::Column::Title,
                        conversation_session::Column::Messages,
                        conversation_session::Column::Status,
                        conversation_session::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError> {
        let results = ConversationSessionEntity::find()
            .filter(ConversationSessionColumn::OperatorId.eq(operator_id))
            .order_by_desc(ConversationSessionColumn::UpdatedAt)
            .all(&self.db)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn delete(&self, session_id: Uuid) -> Result<(), AiError> {
        ConversationSessionEntity::delete_by_id(session_id)
            .exec(&self.db)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

const SESSION_STATUS_ACTIVE: &str = "active";

fn domain_to_model(session: &ConversationSession) -> conversation_session::ActiveModel {
    conversation_session::ActiveModel {
        session_id: sea_orm::ActiveValue::Set(session.session_id),
        operator_id: sea_orm::ActiveValue::Set(session.operator_id),
        title: sea_orm::ActiveValue::Set(session.title.clone()),
        messages: sea_orm::ActiveValue::Set(serde_json::to_value(&session.messages).unwrap_or_default()),
        status: sea_orm::ActiveValue::Set(SESSION_STATUS_ACTIVE.into()),
        created_at: sea_orm::ActiveValue::Set(session.created_at),
        updated_at: sea_orm::ActiveValue::Set(session.updated_at),
    }
}

fn model_to_domain(m: conversation_session::Model) -> Result<ConversationSession, AiError> {
    let messages: Vec<ConversationMessage> = serde_json::from_value(m.messages)
        .map_err(|e| AiError::DatabaseError(format!("Invalid messages JSON: {e}")))?;

    Ok(ConversationSession {
        session_id: m.session_id,
        operator_id: m.operator_id,
        title: m.title,
        messages,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_session() -> ConversationSession {
        ConversationSession {
            session_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            title: "Test session".into(),
            messages: vec![
                ConversationMessage {
                    message_id: Uuid::now_v7(),
                    role: MessageRole::User,
                    content: "Hello".into(),
                    citations: vec![],
                    created_at: Utc::now(),
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let session = sample_session();
        let model = domain_to_model(&session);

        assert_eq!(model.session_id.unwrap(), session.session_id);
        assert_eq!(model.title.unwrap(), "Test session");
        assert_eq!(model.status.unwrap(), "active");
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let session = sample_session();
        let entity = conversation_session::Model {
            session_id: session.session_id,
            operator_id: session.operator_id,
            title: "Loaded session".into(),
            messages: serde_json::to_value(&session.messages).unwrap(),
            status: "active".into(),
            created_at: session.created_at,
            updated_at: session.updated_at,
        };

        let domain = model_to_domain(entity).unwrap();
        assert_eq!(domain.title, "Loaded session");
        assert_eq!(domain.messages.len(), 1);
        assert_eq!(domain.messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_invalid_messages_json() {
        let session = sample_session();
        let entity = conversation_session::Model {
            session_id: session.session_id,
            operator_id: session.operator_id,
            title: "bad".into(),
            messages: serde_json::Value::String("not_json_array".into()),
            status: "active".into(),
            created_at: session.created_at,
            updated_at: session.updated_at,
        };

        let result = model_to_domain(entity);
        assert!(result.is_err());
        assert!(matches!(result, Err(AiError::DatabaseError(_))));
    }
}

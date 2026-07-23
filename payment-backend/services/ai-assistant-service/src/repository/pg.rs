//! PostgreSQL-backed ConversationSessionRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::ConversationSessionRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as SessionActiveModel,
    Column as SessionColumn,
    Entity as SessionEntity,
    Model as SessionModel,
};

pub struct PostgresConversationSessionRepository {
    pub db: DatabaseConnection,
}

impl PostgresConversationSessionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConversationSessionRepository for PostgresConversationSessionRepository {
    async fn load(&self, id: Uuid) -> Result<Option<ConversationSession>, AiError> {
        let result = SessionEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, session: &ConversationSession) -> Result<(), AiError> {
        let model = domain_to_model(session);
        let exists = SessionEntity::find_by_id(session.session_id)
            .one(&self.db)
            .await
            .map_err(|e| AiError::Unavailable(e.to_string()))?
            .is_some();

        if exists {
            SessionEntity::update(SessionActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| AiError::Unavailable(e.to_string()))?;
        } else {
            SessionEntity::insert(SessionActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| AiError::Unavailable(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), AiError> {
        SessionEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ConversationSession>, AiError> {
        let models = SessionEntity::find()
            .filter(SessionColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

fn domain_to_model(s: &ConversationSession) -> SessionModel {
    let messages = serde_json::to_value(&s.messages).unwrap_or_default();
    SessionModel {
        session_id: s.session_id,
        operator_id: s.operator_id,
        title: s.title.clone(),
        messages,
        status: s.status.clone(),
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}

fn model_to_domain(m: SessionModel) -> Result<ConversationSession, AiError> {
    let messages: Vec<Message> = serde_json::from_value(m.messages)
        .map_err(|e| AiError::Unavailable(e.to_string()))?;

    Ok(ConversationSession {
        session_id: m.session_id,
        operator_id: m.operator_id,
        title: m.title,
        messages,
        status: m.status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::domain::aggregates::AiQuery;
use crate::domain::rules::QueryRepository;
use crate::infrastructure::entities::ai_query_entity;
use platform_error::PlatformError;

pub struct PostgresQueryRepository {
    db: DatabaseConnection,
}

impl PostgresQueryRepository {
    pub fn new(db: DatabaseConnection) -> Self { Self { db } }
}

#[async_trait]
impl QueryRepository for PostgresQueryRepository {
    async fn save_query(&self, query: &AiQuery) -> Result<(), PlatformError> {
        let existing = ai_query_entity::Entity::find_by_id(query.query_id)
            .one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut active = ai_query_entity::ActiveModel::from(model);
            active.status = Set(query.status.as_str().to_string());
            active.answer = Set(query.answer.clone());
            active.confidence = Set(query.confidence);
            active.tokens_used = Set(query.tokens_used.map(|t| t as i32));
            active.latency_ms = Set(query.latency_ms.map(|l| l as i64));
            active.completed_at = Set(query.completed_at.map(|dt| dt.into()));
            active.update(&self.db).await
                .map_err(|e| PlatformError::Internal(format!("DB update failed: {e}")))?;
        } else {
            let active = ai_query_entity::ActiveModel {
                query_id: Set(query.query_id),
                session_id: Set(query.session_id.clone()),
                principal_id: Set(query.principal_id),
                query_text: Set(query.query_text.clone()),
                status: Set(query.status.as_str().to_string()),
                answer: Set(query.answer.clone()),
                citations: Set(None),
                confidence: Set(query.confidence),
                tokens_used: Set(query.tokens_used.map(|t| t as i32)),
                latency_ms: Set(query.latency_ms.map(|l| l as i64)),
                created_at: Set(query.created_at.into()),
                completed_at: Set(query.completed_at.map(|dt| dt.into())),
            };
            active.insert(&self.db).await
                .map_err(|e| PlatformError::Internal(format!("DB insert failed: {e}")))?;
        }
        Ok(())
    }

    async fn find_query(&self, id: Uuid) -> Result<Option<AiQuery>, PlatformError> {
        let model = ai_query_entity::Entity::find_by_id(id)
            .one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(model.map(|m| m.into()))
    }
}

impl From<ai_query_entity::Model> for AiQuery {
    fn from(m: ai_query_entity::Model) -> Self {
        use crate::domain::value_objects::QueryStatus;
        AiQuery {
            query_id: m.query_id,
            session_id: m.session_id,
            principal_id: m.principal_id,
            query_text: m.query_text,
            status: match m.status.as_str() {
                "completed" => QueryStatus::Completed,
                "failed" => QueryStatus::Failed,
                _ => QueryStatus::Processing,
            },
            answer: m.answer,
            citations: Vec::new(),
            confidence: m.confidence,
            tokens_used: m.tokens_used.map(|t| t as u32),
            latency_ms: m.latency_ms.map(|l| l as u64),
            created_at: m.created_at.into(),
            completed_at: m.completed_at.map(|dt| dt.into()),
        }
    }
}

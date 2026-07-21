use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use uuid::Uuid;

use crate::domain::aggregates::AiRequest;
use crate::domain::rules::AiRequestRepository;
use crate::infrastructure::entities::ai_request_entity;
use platform_error::PlatformError;

pub struct PostgresAiRequestRepository {
    db: DatabaseConnection,
}

impl PostgresAiRequestRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AiRequestRepository for PostgresAiRequestRepository {
    async fn save(&self, request: &AiRequest) -> Result<(), PlatformError> {
        let existing = ai_request_entity::Entity::find_by_id(request.request_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        if let Some(model) = existing {
            let mut a = ai_request_entity::ActiveModel::from(model);
            a.blocked = Set(request.blocked);
            a.block_reason = Set(request.block_reason.clone());
            a.tokens_used = Set(request.tokens_used.as_ref().map(|t| t.0 as i32));
            a.cost_usd = Set(request.cost_usd.as_ref().map(|c| c.0));
            a.completed_at = Set(request.completed_at.map(|dt| dt.into()));
            a.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        } else {
            let a = ai_request_entity::ActiveModel {
                request_id: Set(request.request_id),
                principal_id: Set(request.principal_id),
                prompt: Set(request.prompt.clone()),
                model: Set(request.model.clone()),
                max_tokens: Set(request.max_tokens.map(|t| t as i32)),
                temperature: Set(request.temperature),
                redacted_prompt: Set(request.redacted_prompt.clone()),
                blocked: Set(request.blocked),
                block_reason: Set(request.block_reason.clone()),
                tokens_used: Set(request.tokens_used.as_ref().map(|t| t.0 as i32)),
                cost_usd: Set(request.cost_usd.as_ref().map(|c| c.0)),
                created_at: Set(request.created_at.into()),
                completed_at: Set(request.completed_at.map(|dt| dt.into())),
            };
            a.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AiRequest>, PlatformError> {
        let model = ai_request_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(model.map(AiRequest::from))
    }

    async fn find_by_principal(
        &self,
        principal_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AiRequest>, PlatformError> {
        let models = ai_request_entity::Entity::find()
            .filter(ai_request_entity::Column::PrincipalId.eq(principal_id))
            .order_by_desc(ai_request_entity::Column::CreatedAt)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(models.into_iter().map(AiRequest::from).collect())
    }
}

impl From<ai_request_entity::Model> for AiRequest {
    fn from(m: ai_request_entity::Model) -> Self {
        use crate::domain::value_objects::{CostUsd, TokenCount};
        AiRequest {
            request_id: m.request_id,
            principal_id: m.principal_id,
            prompt: m.prompt,
            model: m.model,
            max_tokens: m.max_tokens.map(|t| t as u32),
            temperature: m.temperature,
            redacted_prompt: m.redacted_prompt,
            blocked: m.blocked,
            block_reason: m.block_reason,
            block_category: None,
            tokens_used: m.tokens_used.map(|t| TokenCount::new(t as u32)),
            cost_usd: m.cost_usd.map(CostUsd),
            input_tokens: None,
            output_tokens: None,
            latency_ms: None,
            created_at: m.created_at.into(),
            completed_at: m.completed_at.map(|dt| dt.into()),
        }
    }
}

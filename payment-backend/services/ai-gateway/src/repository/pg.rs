//! PostgreSQL-backed AiGatewayRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::AiGatewayRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as QueryActiveModel,
    Column as QueryColumn,
    Entity as QueryEntity,
    Model as QueryModel,
};

pub struct PostgresAiGatewayRepository {
    pub db: DatabaseConnection,
}

impl PostgresAiGatewayRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AiGatewayRepository for PostgresAiGatewayRepository {
    async fn audit_query(&self, query: &AiQuery) -> Result<(), AiGatewayError> {
        let model = QueryModel {
            query_id: query.query_id,
            operator_id: query.operator_id,
            model: query.model.clone(),
            prompt: query.prompt.clone(),
            response: query.response.clone(),
            status: query.status.clone(),
            latency_ms: query.latency_ms as i32,
            created_at: query.created_at,
        };
        QueryEntity::insert(QueryActiveModel::from(model))
            .exec(&self.db)
            .await
            .map_err(|e| AiGatewayError::Unavailable(e.to_string()))?;
        Ok(())
    }

    async fn check_quota(&self, _operator_id: Uuid) -> Result<bool, AiGatewayError> {
        Ok(true) // Quota disabled for initial deployment
    }

    async fn get_circuit_state(&self, _model: &str) -> Result<CircuitState, AiGatewayError> {
        Ok(CircuitState::Closed) // No circuit breaker for initial deployment
    }

    async fn update_circuit_state(&self, _model: &str, _state: CircuitState) -> Result<(), AiGatewayError> {
        Ok(())
    }
}

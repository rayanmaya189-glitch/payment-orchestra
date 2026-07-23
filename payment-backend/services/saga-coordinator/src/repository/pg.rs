//! PostgreSQL-backed SagaRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::SagaRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as SagaActiveModel,
    Column as SagaColumn,
    Entity as SagaEntity,
    Model as SagaModel,
};

pub struct PostgresSagaRepository {
    pub db: DatabaseConnection,
}

impl PostgresSagaRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SagaRepository for PostgresSagaRepository {
    async fn save(&self, saga: &SagaExecution) -> Result<(), SagaError> {
        let model = domain_to_model(saga);
        let exists = SagaEntity::find_by_id(saga.saga_id)
            .one(&self.db)
            .await
            .map_err(|e| SagaError::NotFound(saga.saga_id))?
            .is_some();

        if exists {
            SagaEntity::update(SagaActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SagaError::NotFound(saga.saga_id))?;
        } else {
            SagaEntity::insert(SagaActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SagaError::NotFound(saga.saga_id))?;
        }
        Ok(())
    }

    async fn load(&self, saga_id: Uuid) -> Result<Option<SagaExecution>, SagaError> {
        let result = SagaEntity::find_by_id(saga_id)
            .one(&self.db)
            .await
            .map_err(|e| SagaError::NotFound(saga_id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_stuck_sagas(&self, max_age: chrono::Duration) -> Result<Vec<SagaExecution>, SagaError> {
        let cutoff = Utc::now() - max_age;
        let models = SagaEntity::find()
            .filter(SagaColumn::Status.is_in(vec!["running", "compensating"]))
            .filter(SagaColumn::UpdatedAt.lt(cutoff))
            .all(&self.db)
            .await
            .map_err(|e| SagaError::NotFound(Uuid::default()))?;
        models.into_iter().map(|m| model_to_domain(m)).collect()
    }

    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaExecution>, SagaError> {
        let models = SagaEntity::find()
            .filter(SagaColumn::AggregateId.eq(aggregate_id))
            .all(&self.db)
            .await
            .map_err(|e| SagaError::NotFound(Uuid::default()))?;
        models.into_iter().map(|m| model_to_domain(m)).collect()
    }
}

fn domain_to_model(s: &SagaExecution) -> SagaModel {
    let steps = serde_json::to_value(&s.steps).unwrap_or_default();
    SagaModel {
        saga_id: s.saga_id,
        saga_type: s.saga_type.clone(),
        aggregate_id: s.aggregate_id,
        status: s.status.as_str().to_string(),
        steps,
        current_step: s.current_step,
        compensation_running: s.compensation_running,
        created_at: s.created_at,
        updated_at: s.updated_at,
        completed_at: s.completed_at,
    }
}

fn model_to_domain(m: SagaModel) -> Result<SagaExecution, SagaError> {
    let status = SagaStatus::from_str(&m.status)
        .ok_or_else(|| SagaError::NotFound(m.saga_id))?;
    let steps: Vec<SagaStep> = serde_json::from_value(m.steps)
        .map_err(|e| SagaError::NotFound(m.saga_id))?;

    Ok(SagaExecution {
        saga_id: m.saga_id,
        saga_type: m.saga_type,
        aggregate_id: m.aggregate_id,
        status,
        steps,
        current_step: m.current_step,
        compensation_running: m.compensation_running,
        created_at: m.created_at,
        updated_at: m.updated_at,
        completed_at: m.completed_at,
    })
}

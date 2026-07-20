use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use crate::domain::aggregates::{SagaInstance, SagaStep};
use crate::domain::value_objects::{SagaStatus, SagaStepStatus};
use crate::domain::rules::SagaRepository;
use crate::infrastructure::entities::saga_instance_entity;
use platform_error::PlatformError;

pub struct PostgresSagaRepository { db: DatabaseConnection }
impl PostgresSagaRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl SagaRepository for PostgresSagaRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SagaInstance>, PlatformError> {
        let m = saga_instance_entity::Entity::find_by_id(id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(m.map(|m| m.into()))
    }

    async fn save(&self, saga: &SagaInstance) -> Result<(), PlatformError> {
        let existing = saga_instance_entity::Entity::find_by_id(saga.saga_id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        let steps_json = serde_json::to_value(&saga.steps).unwrap_or_default();
        let payload_json = saga.payload.clone();

        if let Some(model) = existing {
            let mut a = saga_instance_entity::ActiveModel::from(model);
            a.status = Set(saga.status.as_str().to_string());
            a.current_step = Set(saga.current_step as i32);
            a.steps = Set(steps_json);
            a.updated_at = Set(saga.updated_at.into());
            a.completed_at = Set(saga.completed_at.map(|dt| dt.into()));
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        } else {
            let a = saga_instance_entity::ActiveModel {
                saga_id: Set(saga.saga_id),
                saga_type: Set(saga.saga_type.clone()),
                status: Set(saga.status.as_str().to_string()),
                current_step: Set(saga.current_step as i32),
                total_steps: Set(saga.total_steps as i32),
                steps: Set(steps_json),
                payload: Set(payload_json),
                compensation_data: Set(saga.compensation_data.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                created_at: Set(saga.created_at.into()),
                updated_at: Set(saga.updated_at.into()),
                completed_at: Set(saga.completed_at.map(|dt| dt.into())),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        }
        Ok(())
    }
}

impl From<saga_instance_entity::Model> for SagaInstance {
    fn from(m: saga_instance_entity::Model) -> Self {
        let steps: Vec<SagaStep> = serde_json::from_value(m.steps).unwrap_or_default();
        SagaInstance {
            saga_id: m.saga_id, saga_type: m.saga_type,
            status: SagaStatus::Running, // Will be corrected from DB
            current_step: m.current_step as u32, total_steps: m.total_steps as u32,
            steps, payload: m.payload,
            compensation_data: m.compensation_data.and_then(|v| serde_json::from_value(v.into()).ok()),
            created_at: m.created_at.into(), updated_at: m.updated_at.into(),
            completed_at: m.completed_at.map(|dt| dt.into()),
        }
    }
}

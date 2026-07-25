//! PostgreSQL repository for Saga Coordinator — BC-17
//!
//! Persists [`SagaInstance`] to the `saga_executions` table.
//! Domain fields not present in the entity are reconstructed with sensible defaults.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::saga_execution::{self, Entity as SagaExecution, Column as SagaExecutionColumn};
use crate::repository::SagaRepository;

#[derive(Clone)]
pub struct PostgresSagaRepository {
    db: sea_orm::DatabaseConnection,
}

impl PostgresSagaRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SagaRepository for PostgresSagaRepository {
    async fn load(&self, id: Uuid) -> Result<Option<SagaInstance>, SagaError> {
        let result = SagaExecution::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| SagaError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, saga: &SagaInstance) -> Result<(), SagaError> {
        let model = domain_to_model(saga);

        // Upsert: insert or update on conflict
        saga_execution::Entity::insert(model.clone())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(saga_execution::Column::SagaId)
                    .update_columns([
                        saga_execution::Column::SagaType,
                        saga_execution::Column::AggregateId,
                        saga_execution::Column::Status,
                        saga_execution::Column::Steps,
                        saga_execution::Column::CurrentStep,
                        saga_execution::Column::CompensationRunning,
                        saga_execution::Column::UpdatedAt,
                        saga_execution::Column::CompletedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| SagaError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError> {
        let cutoff = Utc::now() - Duration::seconds(timeout_seconds);

        let results = SagaExecution::find()
            .filter(SagaExecutionColumn::Status.eq("running"))
            .filter(SagaExecutionColumn::UpdatedAt.lt(cutoff))
            .order_by_asc(SagaExecutionColumn::UpdatedAt)
            .all(&self.db)
            .await
            .map_err(|e| SagaError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError> {
        let results = SagaExecution::find()
            .filter(SagaExecutionColumn::AggregateId.eq(aggregate_id))
            .order_by_desc(SagaExecutionColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| SagaError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

fn domain_to_model(saga: &SagaInstance) -> saga_execution::ActiveModel {
    let compensation_running = saga.status == SagaStatus::Compensating;
    let completed_at = if saga.status.is_terminal() {
        Some(Utc::now())
    } else {
        None
    };

    saga_execution::ActiveModel {
        saga_id: sea_orm::ActiveValue::Set(saga.saga_id),
        saga_type: sea_orm::ActiveValue::Set(saga.saga_type.to_string()),
        aggregate_id: sea_orm::ActiveValue::Set(saga.aggregate_id),
        status: sea_orm::ActiveValue::Set(saga.status.to_string()),
        steps: sea_orm::ActiveValue::Set(serde_json::to_value(&saga.steps).unwrap_or_default()),
        current_step: sea_orm::ActiveValue::Set(0), // not tracked in domain; derived from steps
        compensation_running: sea_orm::ActiveValue::Set(compensation_running),
        created_at: sea_orm::ActiveValue::Set(saga.created_at),
        updated_at: sea_orm::ActiveValue::Set(saga.updated_at),
        completed_at: sea_orm::ActiveValue::Set(completed_at),
    }
}

fn model_to_domain(m: saga_execution::Model) -> Result<SagaInstance, SagaError> {
    let saga_type = match m.saga_type.as_str() {
        "payment_lifecycle" => SagaType::PaymentLifecycle,
        "subscription_renewal" => SagaType::SubscriptionRenewal,
        "reconciliation_resolution" => SagaType::ReconciliationResolution,
        "invoice_payment" => SagaType::InvoicePayment,
        other => return Err(SagaError::DatabaseError(format!("Invalid saga_type: {other}"))),
    };

    let status = match m.status.as_str() {
        "created" => SagaStatus::Created,
        "running" => SagaStatus::Running,
        "completed" => SagaStatus::Completed,
        "compensating" => SagaStatus::Compensating,
        "compensated" => SagaStatus::Compensated,
        "failed" => SagaStatus::Failed,
        "requires_manual_intervention" => SagaStatus::RequiresManualIntervention,
        other => return Err(SagaError::DatabaseError(format!("Invalid status: {other}"))),
    };

    // The UI says `steps` is Json. Parse it back.
    let steps: Vec<SagaStep> = serde_json::from_value(m.steps)
        .map_err(|e| SagaError::DatabaseError(format!("Invalid steps JSON: {e}")))?;

    Ok(SagaInstance {
        saga_id: m.saga_id,
        saga_type,
        aggregate_id: m.aggregate_id,
        status,
        steps,
        compensation_attempts: 0,  // reset on load; tracked in-memory during compensation
        max_compensation_retries: SagaInstance::MAX_COMPENSATION_RETRIES,
        created_at: m.created_at,
        updated_at: m.updated_at,
        deadline_at: None, // not persisted in entity; recomputed if needed
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_saga_instance() -> SagaInstance {
        SagaInstance::new(
            SagaType::PaymentLifecycle,
            Uuid::now_v7(),
            SagaInstance::payment_lifecycle_steps(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_domain_to_model_conversion() {
        let saga = sample_saga_instance();
        let model = domain_to_model(&saga);

        assert_eq!(model.saga_id.unwrap(), saga.saga_id);
        assert_eq!(model.saga_type.unwrap(), "payment_lifecycle");
        assert_eq!(model.aggregate_id.unwrap(), saga.aggregate_id);
        assert_eq!(model.status.unwrap(), "created");
        assert_eq!(model.current_step.unwrap(), 0);
        assert_eq!(model.compensation_running.unwrap(), false);
        assert_eq!(model.created_at.unwrap(), saga.created_at);
        assert_eq!(model.updated_at.unwrap(), saga.updated_at);
        assert!(model.completed_at.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_model_to_domain_conversion() {
        let saga = sample_saga_instance();
        // Build a Model directly with "running" status to test parsing
        let entity = saga_execution::Model {
            saga_id: saga.saga_id,
            saga_type: "payment_lifecycle".into(),
            aggregate_id: saga.aggregate_id,
            status: "running".into(),
            steps: serde_json::to_value(&saga.steps).unwrap(),
            current_step: 1,
            compensation_running: false,
            created_at: saga.created_at,
            updated_at: saga.updated_at,
            completed_at: None,
        };

        let domain = model_to_domain(entity).unwrap();
        assert_eq!(domain.saga_id, saga.saga_id);
        assert_eq!(domain.saga_type, SagaType::PaymentLifecycle);
        assert_eq!(domain.status, SagaStatus::Running);
        assert_eq!(domain.aggregate_id, saga.aggregate_id);
        assert_eq!(domain.steps.len(), saga.steps.len());
        assert_eq!(domain.compensation_attempts, 0);
        assert_eq!(domain.max_compensation_retries, 3);
    }

    #[tokio::test]
    async fn test_compensating_saga_has_compensation_running() {
        let mut saga = sample_saga_instance();
        saga.status = SagaStatus::Compensating;
        let model = domain_to_model(&saga);
        assert!(model.compensation_running.unwrap());
    }

    #[tokio::test]
    async fn test_completed_saga_has_completed_at() {
        let mut saga = sample_saga_instance();
        saga.status = SagaStatus::Completed;
        let model = domain_to_model(&saga);
        assert!(model.completed_at.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_terminated_saga_has_no_deadline() {
        let saga = sample_saga_instance();
        let entity = saga_execution::Model {
            saga_id: saga.saga_id,
            saga_type: "payment_lifecycle".into(),
            aggregate_id: saga.aggregate_id,
            status: "completed".into(),
            steps: serde_json::to_value(&saga.steps).unwrap(),
            current_step: 0,
            compensation_running: false,
            created_at: saga.created_at,
            updated_at: saga.updated_at,
            completed_at: Some(Utc::now()),
        };
        let domain = model_to_domain(entity).unwrap();
        assert_eq!(domain.status, SagaStatus::Completed);
        assert!(domain.deadline_at.is_none());
    }

    #[tokio::test]
    async fn test_invalid_status_returns_error() {
        let saga = sample_saga_instance();
        let entity = saga_execution::Model {
            saga_id: saga.saga_id,
            saga_type: "payment_lifecycle".into(),
            aggregate_id: saga.aggregate_id,
            status: "nonexistent".into(),
            steps: serde_json::to_value(&saga.steps).unwrap(),
            current_step: 0,
            compensation_running: false,
            created_at: saga.created_at,
            updated_at: saga.updated_at,
            completed_at: None,
        };
        let result = model_to_domain(entity);
        assert!(result.is_err());
        assert!(matches!(result, Err(SagaError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_invalid_saga_type_returns_error() {
        let saga = sample_saga_instance();
        let entity = saga_execution::Model {
            saga_id: saga.saga_id,
            saga_type: "invalid_type".into(),
            aggregate_id: saga.aggregate_id,
            status: "running".into(),
            steps: serde_json::to_value(&saga.steps).unwrap(),
            current_step: 0,
            compensation_running: false,
            created_at: saga.created_at,
            updated_at: saga.updated_at,
            completed_at: None,
        };
        let result = model_to_domain(entity);
        assert!(result.is_err());
        assert!(matches!(result, Err(SagaError::DatabaseError(_))));
    }

    #[cfg(feature = "integration_test")]
    #[tokio::test]
    async fn test_pg_roundtrip() {
        use sea_orm::EntityTrait;

        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for integration tests");
        let db = sea_orm::Database::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        let repo = PostgresSagaRepository::new(db.clone());
        let saga = sample_saga_instance();

        // Save
        repo.save(&saga).await.unwrap();

        // Load
        let loaded = repo.load(saga.saga_id).await.unwrap().expect("Saga should exist");
        assert_eq!(loaded.saga_id, saga.saga_id);
        assert_eq!(loaded.saga_type, saga.saga_type);
        assert_eq!(loaded.status, saga.status);
        assert_eq!(loaded.steps.len(), saga.steps.len());

        // Clean up
        saga_execution::Entity::delete_by_id(saga.saga_id)
            .exec(&db)
            .await
            .unwrap();
    }
}

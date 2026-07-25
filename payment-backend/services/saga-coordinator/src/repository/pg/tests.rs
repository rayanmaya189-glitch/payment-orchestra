//! Tests for Saga Coordinator PostgreSQL repository.

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

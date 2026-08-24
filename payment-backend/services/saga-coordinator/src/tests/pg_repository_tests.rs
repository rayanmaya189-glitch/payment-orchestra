//! PostgreSQL repository integration tests for saga-coordinator.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#![cfg(feature = "integration_test")]

use uuid::Uuid;
use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};

use crate::domain::*;
use migrations::MigratorTrait;

use crate::repository::*;

// ─── Test Helpers ───────────────────────────────────────────────────────────

/// Starts a Postgres container, runs migrations, and returns the repository.
/// Each test gets its own container for full isolation.
struct TestDb {
    _container: testcontainers::ContainerAsync<GenericImage>,
    db: DatabaseConnection,
}

impl TestDb {
    async fn new() -> Self {
        let image = GenericImage::new("postgres", "16-alpine")
            .with_wait_for(WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_PASSWORD", "postgres")
            .with_env_var("POSTGRES_DB", "postgres");

        let container = image
            .start()
            .await
            .expect("Failed to start PostgreSQL container");

        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get host port");

        let url = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = sea_orm::Database::connect(&url)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Run all migrations
        migrations::Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations");

        Self { _container: container, db }
    }

    fn repo(&self) -> PostgresSagaRepository {
        PostgresSagaRepository::new(self.db.clone())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn sample_saga() -> SagaInstance {
    SagaInstance::new(
        SagaType::PaymentLifecycle,
        Uuid::now_v7(),
        SagaInstance::payment_lifecycle_steps(),
    )
    .unwrap()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_save_and_load_saga() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let saga = sample_saga();

    repo.save(&saga).await.unwrap();
    let loaded = repo.load(saga.saga_id).await.unwrap().expect("Saga should exist");

    assert_eq!(loaded.saga_id, saga.saga_id);
    assert_eq!(loaded.saga_type, SagaType::PaymentLifecycle);
    assert_eq!(loaded.status, SagaStatus::Created);
    assert_eq!(loaded.steps.len(), saga.steps.len());
    assert_eq!(loaded.aggregate_id, saga.aggregate_id);
}

#[tokio::test]
async fn test_load_nonexistent_saga() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let result = repo.load(Uuid::now_v7()).await.unwrap();
    assert!(result.is_none(), "Non-existent saga should return None");
}

#[tokio::test]
async fn test_find_by_aggregate() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let aggregate_id = Uuid::now_v7();

    let saga1 = SagaInstance::new(
        SagaType::PaymentLifecycle,
        aggregate_id,
        SagaInstance::payment_lifecycle_steps(),
    )
    .unwrap();
    repo.save(&saga1).await.unwrap();

    let saga2 = SagaInstance::new(
        SagaType::PaymentLifecycle,
        aggregate_id,
        SagaInstance::payment_lifecycle_steps(),
    )
    .unwrap();
    repo.save(&saga2).await.unwrap();

    let found = repo.find_by_aggregate(aggregate_id).await.unwrap();
    assert_eq!(found.len(), 2);
}

#[tokio::test]
async fn test_update_existing_saga() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let mut saga = sample_saga();
    repo.save(&saga).await.unwrap();

    saga.status = SagaStatus::Completed;
    repo.save(&saga).await.unwrap();

    let loaded = repo.load(saga.saga_id).await.unwrap().expect("Saga should exist");
    assert_eq!(loaded.status, SagaStatus::Completed);
}

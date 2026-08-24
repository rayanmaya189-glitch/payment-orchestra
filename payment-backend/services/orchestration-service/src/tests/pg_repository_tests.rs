//! PostgreSQL repository integration tests for orchestration-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#![cfg(feature = "integration_test")]

use uuid::Uuid;
use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};

use crate::domain::*;
use migrations::MigratorTrait;

use crate::repository::*;

// ─── Test Helpers ───────────────────────────────────────────────────────────

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

        migrations::Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations");

        Self { _container: container, db }
    }

    fn repo(&self) -> PostgresOrchestrationRepository {
        PostgresOrchestrationRepository::new(self.db.clone(), None)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn sample_payment_intent(operator_id: Uuid) -> PaymentIntent {
    PaymentIntent {
        payment_intent_id: Uuid::now_v7(),
        operator_id,
        status: PaymentStatus::Created,
        requested_amount: Money { amount_minor_units: 10000, currency: "AED".into() },
        authorized_amount: Money { amount_minor_units: 0, currency: "AED".into() },
        captured_amount: Money { amount_minor_units: 0, currency: "AED".into() },
        refunded_amount: Money { amount_minor_units: 0, currency: "AED".into() },
        currency: "AED".into(),
        idempotency_key: Uuid::now_v7().to_string(),
        payment_method_token_id: None,
        routing_policy_id: None,
        deployment_epoch: 0,
        purpose: PaymentPurpose::Payment,
        metadata: None,
        source_type: None,
        source_id: None,
        risk_score: None,
        risk_level: None,
        expected_settlement_date: None,
        settlement_cycle: None,
        gateway_profile_id: None,
        gateway_profile_version: None,
        gateway_rotation_strategy: None,
        gateway_selection_reason: None,
        routing_attempts: Vec::new(),
        version: 0,
        pending_events: Vec::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ─── PaymentIntent Tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_save_and_load_payment_intent() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let mut intent = sample_payment_intent(operator_id);

    repo.save_payment_intent(&mut intent).await.unwrap();
    let loaded = repo.load_payment_intent(intent.payment_intent_id)
        .await
        .unwrap()
        .expect("PaymentIntent should exist");

    assert_eq!(loaded.payment_intent_id, intent.payment_intent_id);
    assert_eq!(loaded.operator_id, operator_id);
    assert_eq!(loaded.status, PaymentStatus::Created);
    assert_eq!(loaded.requested_amount.amount_minor_units, 10000);
}

#[tokio::test]
async fn test_update_payment_intent_status() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let mut intent = sample_payment_intent(operator_id);

    repo.save_payment_intent(&mut intent).await.unwrap();

    intent.status = PaymentStatus::Authorized;
    intent.authorized_amount = Money { amount_minor_units: 10000, currency: "AED".into() };
    repo.save_payment_intent(&mut intent).await.unwrap();

    let loaded = repo.load_payment_intent(intent.payment_intent_id)
        .await
        .unwrap()
        .expect("PaymentIntent should exist");
    assert_eq!(loaded.status, PaymentStatus::Authorized);
    assert_eq!(loaded.authorized_amount.amount_minor_units, 10000);
}

#[tokio::test]
async fn test_list_payment_intents_for_operator() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();

    let mut intent1 = sample_payment_intent(operator_id);
    repo.save_payment_intent(&mut intent1).await.unwrap();
    let mut intent2 = sample_payment_intent(operator_id);
    repo.save_payment_intent(&mut intent2).await.unwrap();

    let loaded = repo.list_payment_intents_for_operator(operator_id).await.unwrap();
    assert_eq!(loaded.len(), 2);
}

#[tokio::test]
async fn test_payment_intent_isolation() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let op1 = Uuid::now_v7();
    let op2 = Uuid::now_v7();

    let mut i1 = sample_payment_intent(op1);
    repo.save_payment_intent(&mut i1).await.unwrap();
    let mut i2 = sample_payment_intent(op2);
    repo.save_payment_intent(&mut i2).await.unwrap();

    let op1_intents = repo.list_payment_intents_for_operator(op1).await.unwrap();
    assert_eq!(op1_intents.len(), 1);
    assert!(op1_intents[0].operator_id == op1);

    let op2_intents = repo.list_payment_intents_for_operator(op2).await.unwrap();
    assert_eq!(op2_intents.len(), 1);
    assert!(op2_intents[0].operator_id == op2);
}

#[tokio::test]
async fn test_save_with_routing_attempts() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let mut intent = sample_payment_intent(operator_id);

    intent.routing_attempts.push(RoutingAttempt {
        attempt_id: Uuid::now_v7(),
        payment_intent_id: intent.payment_intent_id,
        attempt_number: 1,
        acquirer_link_id: Uuid::now_v7(),
        gateway_profile_id: None,
        gateway_profile_snapshot: None,
        connector_id: "stripe".into(),
        status: AttemptStatus::Declined,
        decline_reason: Some(DeclineReason::DoNotHonor),
        acquirer_reference: None,
        latency_ms: 245,
        fee_calculated: None,
        attempted_at: chrono::Utc::now(),
    });

    repo.save_payment_intent(&mut intent).await.unwrap();
    let loaded = repo.load_payment_intent(intent.payment_intent_id)
        .await
        .unwrap()
        .expect("PaymentIntent should exist");

    assert_eq!(loaded.routing_attempts.len(), 1);
    assert_eq!(loaded.routing_attempts[0].connector_id, "stripe");
    assert_eq!(loaded.routing_attempts[0].status, AttemptStatus::Declined);
    assert_eq!(loaded.routing_attempts[0].decline_reason, Some(DeclineReason::DoNotHonor));
}

#[tokio::test]
async fn test_load_nonexistent_payment_intent() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let result = repo.load_payment_intent(Uuid::now_v7()).await.unwrap();
    assert!(result.is_none());
}

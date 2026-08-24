//! PostgreSQL repository integration tests for subscription-service.
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

    fn repo(&self) -> PostgresSubscriptionRepository {
        PostgresSubscriptionRepository::new(self.db.clone())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn sample_subscription(operator_id: Uuid, customer_id: Uuid) -> Subscription {
    let now = chrono::Utc::now();
    Subscription {
        subscription_id: Uuid::now_v7(),
        operator_id,
        customer_id,
        plan_id: "plan_basic_monthly".into(),
        plan_amount_minor_units: 2999,
        currency: "USD".into(),
        status: SubscriptionStatus::Active,
        current_period_start: now,
        current_period_end: now + chrono::Duration::days(30),
        billing_interval_days: 30,
        payment_method_token_id: None,
        dunning_retry_count: 0,
        max_dunning_retries: 3,
        billing_cycles: vec![],
        dunning_retries: vec![],
        created_at: now,
        cancelled_at: None,
        paused_at: None,
        resumed_at: None,
        pending_events: Vec::new(),
    }
}

fn sample_billing_cycle() -> BillingCycle {
    let now = chrono::Utc::now();
    BillingCycle {
        billing_cycle_id: Uuid::now_v7(),
        period_start: now,
        period_end: now + chrono::Duration::days(30),
        status: BillingCycleStatus::Succeeded,
        payment_intent_id: Some(Uuid::now_v7()),
        idempotency_key: format!("billing-{}", Uuid::now_v7()),
        created_at: now,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_save_and_load_subscription() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let customer_id = Uuid::now_v7();
    let mut sub = sample_subscription(operator_id, customer_id);

    repo.save(&mut sub).await.unwrap();
    let loaded = repo.load(sub.subscription_id).await.unwrap().expect("Subscription should exist");

    assert_eq!(loaded.subscription_id, sub.subscription_id);
    assert_eq!(loaded.operator_id, operator_id);
    assert_eq!(loaded.plan_id, "plan_basic_monthly");
    assert_eq!(loaded.plan_amount_minor_units, 2999);
    assert_eq!(loaded.status, SubscriptionStatus::Active);
}

#[tokio::test]
async fn test_subscription_status_transitions() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let customer_id = Uuid::now_v7();
    let mut sub = sample_subscription(operator_id, customer_id);

    repo.save(&mut sub).await.unwrap();

    // Pause
    sub.status = SubscriptionStatus::Paused;
    sub.paused_at = Some(chrono::Utc::now());
    repo.save(&mut sub).await.unwrap();
    let loaded = repo.load(sub.subscription_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, SubscriptionStatus::Paused);

    // Cancel
    sub.status = SubscriptionStatus::Cancelled;
    sub.cancelled_at = Some(chrono::Utc::now());
    repo.save(&mut sub).await.unwrap();
    let loaded = repo.load(sub.subscription_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, SubscriptionStatus::Cancelled);
}

#[tokio::test]
async fn test_find_by_customer() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let customer_id = Uuid::now_v7();

    let mut sub1 = sample_subscription(operator_id, customer_id);
    repo.save(&mut sub1).await.unwrap();
    let mut sub2 = sample_subscription(operator_id, customer_id);
    repo.save(&mut sub2).await.unwrap();

    let subscriptions = repo.find_by_customer(customer_id).await.unwrap();
    assert_eq!(subscriptions.len(), 2);
}

#[tokio::test]
async fn test_find_by_operator() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();

    let mut sub1 = sample_subscription(operator_id, Uuid::now_v7());
    repo.save(&mut sub1).await.unwrap();
    let mut sub2 = sample_subscription(operator_id, Uuid::now_v7());
    repo.save(&mut sub2).await.unwrap();

    let subscriptions = repo.find_by_operator(operator_id).await.unwrap();
    assert_eq!(subscriptions.len(), 2);
}

#[tokio::test]
async fn test_find_active_for_renewal() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();

    // Due for renewal
    let mut due = sample_subscription(operator_id, Uuid::now_v7());
    due.current_period_end = chrono::Utc::now() - chrono::Duration::hours(1);
    repo.save(&mut due).await.unwrap();

    // Not due
    let mut not_due = sample_subscription(operator_id, Uuid::now_v7());
    not_due.current_period_end = chrono::Utc::now() + chrono::Duration::days(15);
    repo.save(&mut not_due).await.unwrap();

    // Paused
    let mut paused = sample_subscription(operator_id, Uuid::now_v7());
    paused.status = SubscriptionStatus::Paused;
    paused.current_period_end = chrono::Utc::now() - chrono::Duration::hours(1);
    repo.save(&mut paused).await.unwrap();

    let due_for_renewal = repo.find_active_for_renewal().await.unwrap();
    assert!(due_for_renewal.iter().any(|s| s.subscription_id == due.subscription_id));
    assert!(!due_for_renewal.iter().any(|s| s.subscription_id == not_due.subscription_id));
    assert!(!due_for_renewal.iter().any(|s| s.subscription_id == paused.subscription_id));
}

#[tokio::test]
async fn test_subscription_with_billing_cycles() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let operator_id = Uuid::now_v7();
    let customer_id = Uuid::now_v7();
    let mut sub = sample_subscription(operator_id, customer_id);

    sub.billing_cycles.push(sample_billing_cycle());
    sub.billing_cycles.push(sample_billing_cycle());

    repo.save(&mut sub).await.unwrap();
    let loaded = repo.load(sub.subscription_id).await.unwrap().unwrap();

    assert_eq!(loaded.billing_cycles.len(), 2);
}

#[tokio::test]
async fn test_subscription_isolation() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let op1 = Uuid::now_v7();
    let op2 = Uuid::now_v7();

    let mut sub1 = sample_subscription(op1, Uuid::now_v7());
    repo.save(&mut sub1).await.unwrap();
    let mut sub2 = sample_subscription(op2, Uuid::now_v7());
    repo.save(&mut sub2).await.unwrap();

    let op1_subs = repo.find_by_operator(op1).await.unwrap();
    assert_eq!(op1_subs.len(), 1);
    let op2_subs = repo.find_by_operator(op2).await.unwrap();
    assert_eq!(op2_subs.len(), 1);
}

#[tokio::test]
async fn test_load_nonexistent() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let result = repo.load(Uuid::now_v7()).await.unwrap();
    assert!(result.is_none());
}

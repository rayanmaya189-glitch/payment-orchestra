//! PostgreSQL repository integration tests for iam-service.
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

        // Run all migrations to set up the schema
        // Note: in CI, migrations may fail if tables already exist from a previous test.
        migrations::Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations");

        Self { _container: container, db }
    }

    fn repo(&self) -> PostgresIamRepository {
        PostgresIamRepository::new(self.db.clone())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn sample_principal() -> Principal {
    Principal {
        id: Uuid::now_v7(),
        principal_type: PrincipalType::Human,
        email: Some(format!("test_{}@example.com", Uuid::now_v7())),
        password_hash: Some(b"$argon2id$v=19$m=65536,t=3,p=4$salt$hash".to_vec()),
        mfa_enrolled: false,
        mfa_method: None,
        status: PrincipalStatus::Active,
        failed_login_attempts: 0,
        locked_until: None,
        created_at: chrono::Utc::now(),
        last_login_at: None,
        updated_at: chrono::Utc::now(),
    }
}

fn sample_api_key(principal_id: Uuid) -> ApiKey {
    ApiKey {
        api_key_id: Uuid::now_v7(),
        principal_id,
        name: format!("test-key-{}", Uuid::now_v7()),
        key_hash: "test_hash_value_that_is_long_enough".as_bytes().to_vec(),
        scopes: vec!["payments:read".into(), "payments:write".into()],
        status: ApiKeyStatus::Active,
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(90)),
        created_at: chrono::Utc::now(),
        last_used_at: None,
    }
}

fn sample_change(maker_id: Uuid) -> PendingChange {
    PendingChange {
        change_id: Uuid::now_v7(),
        change_type: "update_routing_policy".into(),
        maker_id,
        checker_id: None,
        payload: b"{\"priority\": 1}".to_vec(),
        status: ChangeStatus::Pending,
        maker_note: Some("Update routing priority".into()),
        checker_note: None,
        requested_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(72),
        reviewed_at: None,
    }
}

// ─── Principal Tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_principal_save_and_load() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();

    repo.save_principal(&principal).await.unwrap();
    let loaded = repo.load_principal(principal.id).await.unwrap().expect("Principal should exist");

    assert_eq!(loaded.id, principal.id);
    assert_eq!(loaded.email, principal.email);
    assert_eq!(loaded.status.as_str(), PrincipalStatus::Active.as_str());
}

#[tokio::test]
async fn test_principal_update() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let mut principal = sample_principal();

    repo.save_principal(&principal).await.unwrap();
    principal.status = PrincipalStatus::Suspended;
    principal.failed_login_attempts = 3;
    repo.save_principal(&principal).await.unwrap();

    let loaded = repo.load_principal(principal.id).await.unwrap().expect("Principal should exist");
    assert_eq!(loaded.status.as_str(), PrincipalStatus::Suspended.as_str());
    assert_eq!(loaded.failed_login_attempts, 3);
}

#[tokio::test]
async fn test_find_principal_by_email() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();

    repo.save_principal(&principal).await.unwrap();
    let email = principal.email.clone().expect("Principal should have email");
    let found = repo.find_principal_by_email(&email).await.unwrap().expect("Principal should be found");

    assert_eq!(found.id, principal.id);
}

#[tokio::test]
async fn test_load_nonexistent_principal() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let result = repo.load_principal(Uuid::now_v7()).await.unwrap();
    assert!(result.is_none());
}

// ─── API Key Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_api_key_save_and_load() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();
    repo.save_principal(&principal).await.unwrap();

    let api_key = sample_api_key(principal.id);
    repo.save_api_key(&api_key).await.unwrap();

    let loaded = repo.load_api_key(api_key.api_key_id).await.unwrap().expect("API key should exist");
    assert_eq!(loaded.api_key_id, api_key.api_key_id);
    assert_eq!(loaded.principal_id, principal.id);
    assert_eq!(loaded.status, ApiKeyStatus::Active);
}

#[tokio::test]
async fn test_api_key_revocation() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();
    repo.save_principal(&principal).await.unwrap();

    let mut api_key = sample_api_key(principal.id);
    repo.save_api_key(&api_key).await.unwrap();

    api_key.status = ApiKeyStatus::Revoked;
    repo.save_api_key(&api_key).await.unwrap();

    let loaded = repo.load_api_key(api_key.api_key_id).await.unwrap().expect("API key should exist");
    assert_eq!(loaded.status, ApiKeyStatus::Revoked);
}

#[tokio::test]
async fn test_list_api_keys_for_principal() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();
    repo.save_principal(&principal).await.unwrap();

    let key1 = sample_api_key(principal.id);
    repo.save_api_key(&key1).await.unwrap();
    let key2 = sample_api_key(principal.id);
    repo.save_api_key(&key2).await.unwrap();

    let keys = repo.list_api_keys_for_principal(principal.id).await.unwrap();
    assert_eq!(keys.len(), 2);
}

// ─── PendingChange Tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_pending_change_save_and_load() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let principal = sample_principal();
    repo.save_principal(&principal).await.unwrap();

    let change = sample_change(principal.id);
    repo.save_change(&change).await.unwrap();

    let loaded = repo.load_change(change.change_id).await.unwrap().expect("Change should exist");
    assert_eq!(loaded.change_id, change.change_id);
    assert_eq!(loaded.status, ChangeStatus::Pending);
}

#[tokio::test]
async fn test_pending_change_review() {
    let test_db = TestDb::new().await;
    let repo = test_db.repo();
    let maker = sample_principal();
    repo.save_principal(&maker).await.unwrap();

    let mut change = sample_change(maker.id);
    repo.save_change(&change).await.unwrap();

    change.status = ChangeStatus::Approved;
    change.reviewed_at = Some(chrono::Utc::now());
    repo.save_change(&change).await.unwrap();

    let loaded = repo.load_change(change.change_id).await.unwrap().expect("Change should exist");
    assert_eq!(loaded.status, ChangeStatus::Approved);
    assert!(loaded.reviewed_at.is_some());
}

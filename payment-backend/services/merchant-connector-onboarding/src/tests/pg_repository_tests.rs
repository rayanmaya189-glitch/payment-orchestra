//! PostgreSQL repository integration tests for merchant-connector-onboarding.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;

    use crate::domain::*;
    use crate::repository::PostgresOnboardingRepository;
    use crate::repository::OnboardingRepository;

    struct TestDb {
        _container: ContainerAsync<GenericImage>,
        db: DatabaseConnection,
    }

    impl TestDb {
        async fn new() -> Self {
            let image = GenericImage::new("postgres", "16-alpine")
                .with_env_var("POSTGRES_USER", "test")
                .with_env_var("POSTGRES_PASSWORD", "test")
                .with_env_var("POSTGRES_DB", "test")
                .with_wait_for(WaitFor::message_on_stdout("database system is ready to accept connections"));

            let container = image.start().await.expect("Failed to start PostgreSQL container");
            let port = container.get_host_port_ipv4(5432).await.expect("Failed to get port");
            let database_url = format!("postgres://test:test@127.0.0.1:{}/test", port);

            let db = sea_orm::Database::connect(&database_url)
                .await
                .expect("Failed to connect to PostgreSQL");

            migrations::Migrator::up(&db, None)
                .await
                .expect("Failed to run migrations");

            Self { _container: container, db }
        }

        fn repo(&self) -> PostgresOnboardingRepository {
            PostgresOnboardingRepository::new(self.db.clone())
        }
    }

    fn sample_request(operator_id: uuid::Uuid) -> OnboardingRequest {
        OnboardingRequest::new(
            operator_id,
            "checkout_com".into(),
            "Test Gateway".into(),
            "sandbox".into(),
        )
    }

    #[tokio::test]
    async fn test_save_and_load_onboarding() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let request = sample_request(operator_id);

        repo.save(&request).await.expect("Failed to save onboarding request");
        let loaded = repo.load(request.link_id).await.expect("Failed to load");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.link_id, request.link_id);
        assert_eq!(loaded.operator_id, operator_id);
        assert_eq!(loaded.connector_id, "checkout_com");
        assert_eq!(loaded.environment, "sandbox");
        assert_eq!(loaded.status, OnboardingStatus::Draft);
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let r1 = sample_request(operator_id);
        repo.save(&r1).await.expect("Failed to save r1");

        let r2 = sample_request(operator_id);
        repo.save(&r2).await.expect("Failed to save r2");

        let results = repo.find_by_operator(operator_id).await.expect("Failed to find by operator");
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_find_active_onboardings() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        // Draft request (not active)
        let draft = sample_request(operator_id);
        repo.save(&draft).await.expect("Failed to save draft");

        // Create an active request manually (simulating full lifecycle)
        let mut active = OnboardingRequest::new(
            operator_id,
            "network_international".into(),
            "Active Gateway".into(),
            "sandbox".into(),
        );
        // Simulate the active state by creating with the right status
        // Since OnboardingRequest::new() creates Draft, we need to go through the lifecycle
        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "network_international").unwrap();
        let mut credentials = std::collections::HashMap::new();
        credentials.insert("merchant_id".into(), "MER-12345".into());
        credentials.insert("api_key".into(), "abc123def456abc123def456abc12345".into());
        credentials.insert("environment".into(), "sandbox".into());
        active.submit_credentials(credentials, schema).expect("Failed to submit credentials");
        active.start_test().expect("Failed to start test");
        active.record_test_success(ConnectionTestResult {
            success: true,
            latency_ms: 100,
            error: None,
            merchant_name: Some("Test Merchant".into()),
            permissions: vec!["authorize".into()],
        }).expect("Failed to record test success");
        repo.save(&active).await.expect("Failed to save active");

        let active_results = repo.find_active(operator_id).await.expect("Failed to find active");
        assert_eq!(active_results.len(), 1);
        assert_eq!(active_results[0].link_id, active.link_id);
        assert_eq!(active_results[0].status, OnboardingStatus::Active);
    }

    #[tokio::test]
    async fn test_update_onboarding_status() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let mut request = sample_request(operator_id);

        repo.save(&request).await.expect("Failed to save");

        // Submit credentials and activate
        let connectors = default_connectors();
        let schema = connectors.iter().find(|c| c.connector_id == "checkout_com").unwrap();
        let mut credentials = std::collections::HashMap::new();
        credentials.insert("secret_key".into(), "sk_test_abc".into());
        credentials.insert("public_key".into(), "pk_test_xyz".into());
        credentials.insert("environment".into(), "sandbox".into());
        request.submit_credentials(credentials, schema).expect("Failed to submit credentials");
        repo.save(&request).await.expect("Failed to save updated");

        let loaded = repo.load(request.link_id).await.expect("Failed to load")
            .expect("Request should exist");
        assert_eq!(loaded.status, OnboardingStatus::CredentialsSubmitted);
    }

    #[tokio::test]
    async fn test_load_nonexistent_onboarding() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }
}

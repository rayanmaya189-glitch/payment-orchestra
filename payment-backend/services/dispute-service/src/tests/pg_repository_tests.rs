//! PostgreSQL repository integration tests for dispute-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;

    use crate::domain::*;
    use crate::repository::PostgresDisputeRepository;
    use crate::repository::DisputeRepository;

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

        fn repo(&self) -> PostgresDisputeRepository {
            PostgresDisputeRepository::new(self.db.clone())
        }
    }

    fn sample_case() -> ChargebackCase {
        ChargebackCase::new(
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "fraud".into(),
            5000,
            "AED".into(),
        )
        .expect("Failed to create sample case")
    }

    #[tokio::test]
    async fn test_save_and_load_case() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let case = sample_case();

        repo.save(&case).await.expect("Failed to save case");

        let loaded = repo.load(case.chargeback_id).await.expect("Failed to load case");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.chargeback_id, case.chargeback_id);
        assert_eq!(loaded.status, ChargebackStatus::Received);
        assert_eq!(loaded.reason_code, "fraud");
        assert_eq!(loaded.amount_minor_units, 5000);
        assert_eq!(loaded.currency, "AED");
        assert!(loaded.submissions.is_empty());
        assert!(loaded.resolved_at.is_none());
    }

    #[tokio::test]
    async fn test_find_by_payment_intent() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let payment_intent_id = uuid::Uuid::now_v7();

        let case1 = ChargebackCase::new(
            uuid::Uuid::now_v7(),
            payment_intent_id,
            uuid::Uuid::now_v7(),
            "fraud".into(),
            5000,
            "AED".into(),
        ).unwrap();
        repo.save(&case1).await.expect("Failed to save case1");

        let case2 = ChargebackCase::new(
            uuid::Uuid::now_v7(),
            payment_intent_id,
            uuid::Uuid::now_v7(),
            "duplicate".into(),
            3000,
            "AED".into(),
        ).unwrap();
        repo.save(&case2).await.expect("Failed to save case2");

        let results = repo.find_by_payment_intent(payment_intent_id)
            .await
            .expect("Failed to find by payment intent");
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let case1 = ChargebackCase::new(
            operator_id,
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "fraud".into(),
            5000,
            "AED".into(),
        ).unwrap();
        repo.save(&case1).await.expect("Failed to save case1");

        let case2 = ChargebackCase::new(
            operator_id,
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "duplicate".into(),
            3000,
            "AED".into(),
        ).unwrap();
        repo.save(&case2).await.expect("Failed to save case2");

        let results = repo.find_by_operator(operator_id)
            .await
            .expect("Failed to find by operator");
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_find_open_cases() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        // Create an open case (Received status)
        let open_case = ChargebackCase::new(
            operator_id,
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "fraud".into(),
            5000,
            "AED".into(),
        ).unwrap();
        repo.save(&open_case).await.expect("Failed to save open case");

        // Create a resolved case
        let mut resolved_case = ChargebackCase::new(
            operator_id,
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "duplicate".into(),
            3000,
            "AED".into(),
        ).unwrap();
        resolved_case.status = ChargebackStatus::Won;
        resolved_case.resolved_at = Some(chrono::Utc::now());
        resolved_case.outcome = Some(ChargebackOutcome::Won);
        repo.save(&resolved_case).await.expect("Failed to save resolved case");

        let open_cases = repo.find_open_cases(operator_id)
            .await
            .expect("Failed to find open cases");
        assert_eq!(open_cases.len(), 1);
        assert_eq!(open_cases[0].chargeback_id, open_case.chargeback_id);
    }

    #[tokio::test]
    async fn test_update_case_with_submission() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let mut case = sample_case();

        repo.save(&case).await.expect("Failed to save case");

        // Submit representment
        let evidence = RepresentmentEvidence {
            transaction_receipt: Some(uuid::Uuid::now_v7()),
            delivery_confirmation: None,
            customer_communication: None,
            cardholder_agreement: None,
            refund_policy: None,
            description: "Customer received the product. See attached receipt.".into(),
            supporting_documents: vec![uuid::Uuid::now_v7()],
        };
        case.submit_representment(evidence).expect("Failed to submit representment");
        repo.save(&case).await.expect("Failed to save updated case");

        let loaded = repo.load(case.chargeback_id).await.expect("Failed to load")
            .expect("Case should exist");
        assert_eq!(loaded.status, ChargebackStatus::RepresentmentSubmitted);
        assert_eq!(loaded.submissions.len(), 1);
        assert_eq!(loaded.submissions[0].evidence.description, "Customer received the product. See attached receipt.");
    }

    #[tokio::test]
    async fn test_load_nonexistent_case() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }
}

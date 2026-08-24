//! PostgreSQL repository integration tests for payment-link-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;
    use chrono::{Utc, Duration};

    use crate::domain::*;
    use crate::repository::PostgresPaymentLinkRepository;
    use crate::repository::PaymentLinkRepository;

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

        fn repo(&self) -> PostgresPaymentLinkRepository {
            PostgresPaymentLinkRepository::new(self.db.clone())
        }
    }

    fn sample_link(operator_id: uuid::Uuid) -> PaymentLink {
        PaymentLink::new(
            operator_id,
            "plink_test_token_123".into(),
            5000,
            "AED".into(),
            Some("Test payment link".into()),
            None,
            Utc::now() + Duration::days(30),
        ).expect("Failed to create sample link")
    }

    #[tokio::test]
    async fn test_save_and_load_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let link = sample_link(operator_id);

        repo.save(&link).await.expect("Failed to save link");
        let loaded = repo.load(link.payment_link_id).await.expect("Failed to load link");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.payment_link_id, link.payment_link_id);
        assert_eq!(loaded.token, "plink_test_token_123");
        assert_eq!(loaded.amount_minor_units, 5000);
        assert_eq!(loaded.currency, "AED");
        assert_eq!(loaded.status, PaymentLinkStatus::Active);
    }

    #[tokio::test]
    async fn test_load_by_token() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let link = sample_link(operator_id);

        repo.save(&link).await.expect("Failed to save link");
        let found = repo.load_by_token("plink_test_token_123").await.expect("Failed to load by token");
        assert!(found.is_some());
        assert_eq!(found.unwrap().payment_link_id, link.payment_link_id);

        let not_found = repo.load_by_token("nonexistent_token").await.expect("Failed query");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let link1 = sample_link(operator_id);
        repo.save(&link1).await.expect("Failed to save link1");

        let link2 = sample_link(operator_id);
        repo.save(&link2).await.expect("Failed to save link2");

        let links = repo.find_by_operator(operator_id).await.expect("Failed to find by operator");
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_find_expired_links() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        // Create an already-expired link
        let expired_link = PaymentLink::new(
            operator_id,
            "plink_expired_001".into(),
            5000,
            "AED".into(),
            None,
            None,
            Utc::now() - Duration::hours(1), // expires in the past
        ).expect("Failed to create expired link");
        repo.save(&expired_link).await.expect("Failed to save expired link");

        // Create a valid link
        let valid_link = sample_link(operator_id);
        repo.save(&valid_link).await.expect("Failed to save valid link");

        let expired = repo.find_expired().await.expect("Failed to find expired");
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].payment_link_id, expired_link.payment_link_id);
    }

    #[tokio::test]
    async fn test_update_link_status() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let mut link = sample_link(operator_id);

        repo.save(&link).await.expect("Failed to save link");

        link.status = PaymentLinkStatus::Used;
        link.used_at = Some(Utc::now());
        link.payment_intent_id = Some(uuid::Uuid::now_v7());
        repo.save(&link).await.expect("Failed to update link");

        let loaded = repo.load(link.payment_link_id).await.expect("Failed to load updated")
            .expect("Link should exist");
        assert_eq!(loaded.status, PaymentLinkStatus::Used);
        assert!(loaded.used_at.is_some());
        assert!(loaded.payment_intent_id.is_some());
    }

    #[tokio::test]
    async fn test_load_nonexistent_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }
}

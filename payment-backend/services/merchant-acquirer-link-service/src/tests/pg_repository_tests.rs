//! PostgreSQL repository integration tests for merchant-acquirer-link-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;

    use crate::domain::*;
    use crate::repository::PostgresLinkRepository;
    use crate::repository::LinkRepository;

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

        fn repo(&self) -> PostgresLinkRepository {
            PostgresLinkRepository::new(self.db.clone())
        }
    }

    fn sample_link(operator_id: uuid::Uuid) -> MerchantAcquirerLink {
        MerchantAcquirerLink::new(
            operator_id,
            "checkout_com".into(),
            "Test Gateway".into(),
            LinkEnvironment::Sandbox,
            vec![1, 2, 3, 4],
            "hash_abc".into(),
        )
    }

    #[tokio::test]
    async fn test_save_and_load_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let link = sample_link(uuid::Uuid::now_v7());

        repo.save(&link).await.expect("Failed to save link");
        let loaded = repo.load(link.link_id).await.expect("Failed to load link");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.link_id, link.link_id);
        assert_eq!(loaded.display_name, "Test Gateway");
        assert_eq!(loaded.environment, LinkEnvironment::Sandbox);
        assert_eq!(loaded.status, LinkStatus::Testing);
        assert_eq!(loaded.health_status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let link1 = sample_link(operator_id);
        repo.save(&link1).await.expect("Failed to save link1");

        let mut link2 = sample_link(operator_id);
        link2.connector_id = "stripe".into();
        link2.display_name = "Stripe Gateway".into();
        repo.save(&link2).await.expect("Failed to save link2");

        let links = repo.find_by_operator(operator_id).await.expect("Failed to find by operator");
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_connector() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let link1 = sample_link(operator_id);
        repo.save(&link1).await.expect("Failed to save link1");

        let mut link2 = MerchantAcquirerLink::new(
            uuid::Uuid::now_v7(),
            "checkout_com".into(),
            "Second Gateway".into(),
            LinkEnvironment::Production,
            vec![5, 6, 7, 8],
            "hash_xyz".into(),
        );
        repo.save(&link2).await.expect("Failed to save link2");

        let links = repo.find_by_connector("checkout_com").await.expect("Failed to find by connector");
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_credentials_hash() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let link = sample_link(uuid::Uuid::now_v7());

        repo.save(&link).await.expect("Failed to save link");
        let found = repo.find_by_credentials_hash("hash_abc").await.expect("Failed to find by hash");
        assert!(found.is_some());
        assert_eq!(found.unwrap().link_id, link.link_id);

        let not_found = repo.find_by_credentials_hash("nonexistent_hash").await.expect("Failed query");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_load_nonexistent_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let mut link = sample_link(uuid::Uuid::now_v7());

        repo.save(&link).await.expect("Failed to save link");

        link.display_name = "Updated Gateway".into();
        link.status = LinkStatus::Active;
        link.health_status = HealthStatus::Healthy;
        repo.save(&link).await.expect("Failed to update link");

        let loaded = repo.load(link.link_id).await.expect("Failed to load updated link")
            .expect("Link should exist");
        assert_eq!(loaded.display_name, "Updated Gateway");
        assert_eq!(loaded.status, LinkStatus::Active);
        assert_eq!(loaded.health_status, HealthStatus::Healthy);
    }
}

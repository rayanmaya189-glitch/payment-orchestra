//! PostgreSQL repository integration tests for outbox-relay.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;
    use chrono::Utc;

    use crate::domain::*;
    use crate::repository::PostgresOutboxRepository;
    use crate::repository::OutboxRepository;

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

        fn repo(&self) -> PostgresOutboxRepository {
            PostgresOutboxRepository::new(self.db.clone())
        }
    }

    fn sample_entry() -> OutboxEntry {
        OutboxEntry {
            outbox_id: uuid::Uuid::now_v7(),
            aggregate_type: "PaymentIntent".into(),
            aggregate_id: uuid::Uuid::now_v7(),
            event_type: "PaymentAuthorized".into(),
            event_version: 1,
            payload: b"{\"key\":\"value\"}".to_vec(),
            created_at: Utc::now(),
            published_at: None,
        }
    }

    #[tokio::test]
    async fn test_save_and_load_entry() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let entry = sample_entry();

        repo.save_entry(&entry).await.expect("Failed to save entry");

        let loaded = repo.load_entry(entry.outbox_id).await.expect("Failed to load entry");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.outbox_id, entry.outbox_id);
        assert_eq!(loaded.event_type, "PaymentAuthorized");
        assert_eq!(loaded.aggregate_type, "PaymentIntent");
        assert!(loaded.published_at.is_none());
    }

    #[tokio::test]
    async fn test_find_unpublished_entries() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();

        let entry1 = sample_entry();
        repo.save_entry(&entry1).await.expect("Failed to save entry1");

        let entry2 = sample_entry();
        repo.save_entry(&entry2).await.expect("Failed to save entry2");

        let unpublished = repo.find_unpublished(100).await.expect("Failed to find unpublished");
        assert_eq!(unpublished.len(), 2);
    }

    #[tokio::test]
    async fn test_mark_published() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let entry = sample_entry();

        repo.save_entry(&entry).await.expect("Failed to save entry");
        repo.mark_published(entry.outbox_id).await.expect("Failed to mark published");

        let unpublished = repo.find_unpublished(100).await.expect("Failed to find unpublished");
        assert_eq!(unpublished.len(), 0);
    }

    #[tokio::test]
    async fn test_count_unpublished() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();

        let count_before = repo.count_unpublished().await.expect("Failed to count");
        assert_eq!(count_before, 0);

        let entry = sample_entry();
        repo.save_entry(&entry).await.expect("Failed to save entry");

        let count_after = repo.count_unpublished().await.expect("Failed to count after");
        assert_eq!(count_after, 1);
    }

    #[tokio::test]
    async fn test_list_entries_returns_all() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();

        repo.save_entry(&sample_entry()).await.expect("Failed to save entry1");
        repo.save_entry(&sample_entry()).await.expect("Failed to save entry2");

        let entries = repo.list_entries().await.expect("Failed to list entries");
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent_entry() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load_entry(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mark_nonexistent_published_returns_error() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.mark_published(uuid::Uuid::now_v7()).await;
        assert!(result.is_err());
    }
}

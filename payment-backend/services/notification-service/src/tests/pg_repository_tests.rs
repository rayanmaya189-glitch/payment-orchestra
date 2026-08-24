//! PostgreSQL repository integration tests for notification-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;
    use chrono::Utc;

    use crate::domain::*;
    use crate::repository::PostgresNotificationRepository;
    use crate::repository::NotificationRepository;

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

        fn repo(&self) -> PostgresNotificationRepository {
            PostgresNotificationRepository::new(self.db.clone())
        }
    }

    fn sample_notification(operator_id: uuid::Uuid) -> NotificationRequest {
        NotificationRequest::new(
            operator_id,
            NotificationChannel::Email,
            "merchant@example.com".into(),
            "payment_failed".into(),
            r#"{"amount": "5000 AED"}"#.into(),
            Some("Payment Failed".into()),
        )
    }

    #[tokio::test]
    async fn test_save_and_load_notification() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let notification = sample_notification(operator_id);

        repo.save(&notification).await.expect("Failed to save notification");
        let loaded = repo.load(notification.notification_id).await.expect("Failed to load");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.notification_id, notification.notification_id);
        assert_eq!(loaded.channel, NotificationChannel::Email);
        assert_eq!(loaded.recipient, "merchant@example.com");
        assert_eq!(loaded.template_id, "payment_failed");
        assert_eq!(loaded.status, DeliveryStatus::Queued);
        assert_eq!(loaded.retry_count, 0);
    }

    #[tokio::test]
    async fn test_find_pending_notifications() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        // Queued notification
        let n1 = sample_notification(operator_id);
        repo.save(&n1).await.expect("Failed to save n1");

        // Sent notification (should not appear in pending)
        let mut n2 = sample_notification(operator_id);
        n2.status = DeliveryStatus::Sent;
        n2.sent_at = Some(Utc::now());
        repo.save(&n2).await.expect("Failed to save n2");

        let pending = repo.find_pending().await.expect("Failed to find pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].notification_id, n1.notification_id);
        assert_eq!(pending[0].status, DeliveryStatus::Queued);
    }

    #[tokio::test]
    async fn test_find_dead_letter_notifications() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        // Dead letter notification
        let mut dead = sample_notification(operator_id);
        dead.status = DeliveryStatus::DeadLetter;
        dead.retry_count = 3;
        dead.last_error = Some("Max retries exceeded".into());
        repo.save(&dead).await.expect("Failed to save dead letter");

        // Normal notification
        let normal = sample_notification(operator_id);
        repo.save(&normal).await.expect("Failed to save normal");

        let dead_letters = repo.find_dead_letter().await.expect("Failed to find dead letters");
        assert_eq!(dead_letters.len(), 1);
        assert_eq!(dead_letters[0].notification_id, dead.notification_id);
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();

        let n1 = sample_notification(operator_id);
        repo.save(&n1).await.expect("Failed to save n1");

        let n2 = sample_notification(operator_id);
        repo.save(&n2).await.expect("Failed to save n2");

        let results = repo.find_by_operator(operator_id).await.expect("Failed to find by operator");
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_update_notification_status() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = uuid::Uuid::now_v7();
        let mut notification = sample_notification(operator_id);

        repo.save(&notification).await.expect("Failed to save");

        // Update to failed
        notification.status = DeliveryStatus::Failed;
        notification.retry_count = 1;
        notification.last_error = Some("SMTP error".into());
        repo.save(&notification).await.expect("Failed to update");

        let loaded = repo.load(notification.notification_id).await.expect("Failed to load")
            .expect("Notification should exist");
        assert_eq!(loaded.status, DeliveryStatus::Failed);
        assert_eq!(loaded.retry_count, 1);
        assert_eq!(loaded.last_error, Some("SMTP error".into()));
    }

    #[tokio::test]
    async fn test_load_nonexistent_notification() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(uuid::Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }
}

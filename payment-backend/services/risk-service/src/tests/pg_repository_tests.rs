//! PostgreSQL repository integration tests for risk-service.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;

    use crate::domain::*;
    use crate::repository::PostgresRiskRepository;
    use crate::repository::RiskRepository;

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

        fn repo(&self) -> PostgresRiskRepository {
            PostgresRiskRepository::new(self.db.clone())
        }
    }

    fn sample_assessment(payment_intent_id: uuid::Uuid) -> RiskAssessment {
        let mut a = RiskAssessment::new(payment_intent_id);
        a.risk_score = 0.85;
        a.risk_level = RiskLevel::High;
        a.risk_factors = vec!["High amount".into(), "Geo mismatch".into()];
        a
    }

    #[tokio::test]
    async fn test_save_and_load_assessment() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let payment_intent_id = uuid::Uuid::now_v7();
        let assessment = sample_assessment(payment_intent_id);

        repo.save(&assessment).await.expect("Failed to save assessment");

        let loaded = repo.load_by_payment_intent(payment_intent_id)
            .await
            .expect("Failed to load assessment")
            .expect("Assessment should exist");

        assert_eq!(loaded.risk_assessment_id, assessment.risk_assessment_id);
        assert_eq!(loaded.payment_intent_id, payment_intent_id);
        assert!((loaded.risk_score - 0.85).abs() < f64::EPSILON);
        assert_eq!(loaded.risk_level, RiskLevel::High);
        assert_eq!(loaded.risk_factors.len(), 2);
    }

    #[tokio::test]
    async fn test_save_and_load_multiple_assessments() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let payment_intent_id = uuid::Uuid::now_v7();

        let a1 = sample_assessment(payment_intent_id);
        repo.save(&a1).await.expect("Failed to save a1");

        let a2 = sample_assessment(uuid::Uuid::now_v7());
        repo.save(&a2).await.expect("Failed to save a2");

        // Should find the latest assessment for the payment intent
        let loaded = repo.load_by_payment_intent(payment_intent_id)
            .await
            .expect("Failed to load")
            .expect("Assessment should exist");
        assert_eq!(loaded.risk_assessment_id, a1.risk_assessment_id);
    }

    #[tokio::test]
    async fn test_find_high_risk() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        use chrono::{Utc, Duration};

        let payment_intent_id = uuid::Uuid::now_v7();
        let assessment = sample_assessment(payment_intent_id);
        repo.save(&assessment).await.expect("Failed to save assessment");

        let since = Utc::now() - Duration::hours(1);
        let high_risk = repo.find_high_risk(uuid::Uuid::now_v7(), since)
            .await
            .expect("Failed to find high risk");

        assert!(!high_risk.is_empty());
        assert!(high_risk.iter().any(|a| a.risk_level == RiskLevel::High));
    }

    #[tokio::test]
    async fn test_load_nonexistent_assessment() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load_by_payment_intent(uuid::Uuid::now_v7())
            .await
            .expect("Failed query");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_risk_stats_empty() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let stats = repo.get_risk_stats(uuid::Uuid::now_v7(), 24)
            .await
            .expect("Failed to get stats");
        assert!((stats.avg_risk_score - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.high_risk_count, 0);
    }
}

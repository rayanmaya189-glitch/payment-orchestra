//! PostgreSQL repository integration tests for connector-gateway.
//! Uses testcontainers to spin up a real PostgreSQL instance and runs migrations.

#[cfg(feature = "integration_test")]
mod integration {
    use sea_orm::DatabaseConnection;
    use testcontainers::{GenericImage, ContainerAsync, ImageExt, runners::AsyncRunner, core::WaitFor};
    use migrations::MigratorTrait;
    use uuid::Uuid;

    use crate::domain::*;
    use crate::repository::PostgresConnectorGatewayRepository;
    use crate::repository::GatewayProfileRepository;

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

        fn repo(&self) -> PostgresConnectorGatewayRepository {
            PostgresConnectorGatewayRepository::new(self.db.clone())
        }
    }

    fn sample_profile(operator_id: Uuid, link_id: Uuid) -> GatewayProfile {
        GatewayProfile {
            profile_id: Uuid::now_v7(),
            operator_id,
            connector_id: "stripe".into(),
            merchant_acquirer_link_id: link_id,
            status: ProfileStatus::Active,
            limits: TransactionLimits {
                min_amount_minor: 100,
                max_amount_minor: 50_000_000,
                daily_volume_limit_minor: 5_000_000_000,
                monthly_volume_limit_minor: 50_000_000_000,
                max_refund_amount_minor: 5_000_000,
            },
            fees: FeeStructure {
                fixed_fee_minor: 100,
                percentage_fee_bps: 250,
                cross_border_fee_bps: 50,
                currency_conversion_fee_bps: 75,
                max_fee_cap: Some(10_000),
                min_fee_floor: Some(100),
                tiered_pricing: None,
            },
            routing_priority: 1,
            enabled_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            enabled_currencies: vec!["AED".into(), "USD".into()],
            enabled_countries: vec!["AE".into()],
            rate_limits: RateLimitConfig {
                per_second: 100,
                per_day: 100_000,
                burst_size: 200,
            },
            monitoring: MonitoringThresholds {
                success_rate_alert: 0.95,
                success_rate_critical: 0.90,
                latency_p99_alert_ms: 3000,
                latency_p99_critical_ms: 5000,
                auto_disable_on_low_success: false,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_save_and_load_profile() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();
        let profile = sample_profile(operator_id, link_id);

        repo.save(&profile).await.expect("Failed to save profile");
        let loaded = repo.load(profile.profile_id).await.expect("Failed to load profile");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.profile_id, profile.profile_id);
        assert_eq!(loaded.operator_id, operator_id);
        assert_eq!(loaded.connector_id, "stripe");
        assert_eq!(loaded.status, ProfileStatus::Active);
        assert_eq!(loaded.routing_priority, 1);
    }

    #[tokio::test]
    async fn test_find_active_for_operator() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();

        let p1 = sample_profile(operator_id, Uuid::now_v7());
        repo.save(&p1).await.expect("Failed to save p1");

        let mut p2 = sample_profile(operator_id, Uuid::now_v7());
        p2.connector_id = "checkout_com".into();
        p2.routing_priority = 2;
        repo.save(&p2).await.expect("Failed to save p2");

        let profiles = repo.find_active_for_operator(operator_id).await.expect("Failed to find active");
        assert_eq!(profiles.len(), 2);
        // Should be sorted by routing_priority
        assert_eq!(profiles[0].routing_priority, 1);
        assert_eq!(profiles[1].routing_priority, 2);
    }

    #[tokio::test]
    async fn test_find_by_connector() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();

        let p1 = sample_profile(operator_id, Uuid::now_v7());
        repo.save(&p1).await.expect("Failed to save p1");

        let profiles = repo.find_by_connector("stripe").await.expect("Failed to find by connector");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].connector_id, "stripe");

        let not_found = repo.find_by_connector("nonexistent").await.expect("Failed query");
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_link() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let profile = sample_profile(operator_id, link_id);
        repo.save(&profile).await.expect("Failed to save profile");

        let found = repo.find_by_link(link_id).await.expect("Failed to find by link");
        assert!(found.is_some());
        assert_eq!(found.unwrap().merchant_acquirer_link_id, link_id);
    }

    #[tokio::test]
    async fn test_list_all_profiles() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();

        repo.save(&sample_profile(operator_id, Uuid::now_v7())).await.expect("Failed to save p1");
        repo.save(&sample_profile(Uuid::now_v7(), Uuid::now_v7())).await.expect("Failed to save p2");

        let all = repo.list_all().await.expect("Failed to list all");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nonexistent_profile() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let result = repo.load(Uuid::now_v7()).await.expect("Failed query");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_profile() {
        let test_db = TestDb::new().await;
        let repo = test_db.repo();
        let operator_id = Uuid::now_v7();
        let mut profile = sample_profile(operator_id, Uuid::now_v7());

        repo.save(&profile).await.expect("Failed to save profile");

        profile.status = ProfileStatus::Disabled;
        profile.routing_priority = 5;
        repo.save(&profile).await.expect("Failed to update profile");

        let loaded = repo.load(profile.profile_id).await.expect("Failed to load updated")
            .expect("Profile should exist");
        assert_eq!(loaded.status, ProfileStatus::Disabled);
        assert_eq!(loaded.routing_priority, 5);
    }
}

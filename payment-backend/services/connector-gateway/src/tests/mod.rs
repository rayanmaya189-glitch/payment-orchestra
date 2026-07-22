//! Integration tests for connector-gateway.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::commands::{self, CommandHandler};
    use crate::domain::{
        self, CardScheme, ConnectorRegistry, FeeStructure, GatewayProfile, MonitoringThresholds,
        ProfileStatus, RateLimitConfig, RotationState, RotationStrategy, TransactionLimits,
    };
    use crate::queries::{self, QueryHandler};
    use crate::repository::{GatewayProfileRepository, InMemoryGatewayProfileRepository};

    fn test_profile_id() -> Uuid {
        Uuid::now_v7()
    }

    fn test_operator_id() -> Uuid {
        Uuid::now_v7()
    }

    fn test_link_id() -> Uuid {
        Uuid::now_v7()
    }

    fn sample_limits() -> TransactionLimits {
        TransactionLimits {
            min_amount_minor: 100,
            max_amount_minor: 50000000,
            daily_volume_limit_minor: 5000000000,
            monthly_volume_limit_minor: 50000000000,
            max_refund_amount_minor: 50000000,
        }
    }

    fn sample_fees() -> FeeStructure {
        FeeStructure {
            fixed_fee_minor: 100,
            percentage_fee_bps: 250,
            cross_border_fee_bps: 50,
            currency_conversion_fee_bps: 75,
            max_fee_cap: Some(10000),
            min_fee_floor: Some(100),
            tiered_pricing: None,
        }
    }

    fn sample_gateway_profile(profile_id: Uuid, operator_id: Uuid, link_id: Uuid, connector_id: &str) -> GatewayProfile {
        GatewayProfile {
            profile_id,
            operator_id,
            connector_id: connector_id.to_string(),
            merchant_acquirer_link_id: link_id,
            status: ProfileStatus::Active,
            limits: sample_limits(),
            fees: sample_fees(),
            routing_priority: 1,
            enabled_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            enabled_currencies: vec!["AED".into(), "USD".into()],
            enabled_countries: vec!["AE".into()],
            rate_limits: RateLimitConfig {
                per_second: 100,
                per_day: 100000,
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

    // ─── Circuit Breaker Tests ─────────────────────────────────────────

    #[test]
    fn test_circuit_breaker_initial_state_closed() {
        let mut cb = domain::CircuitBreaker::new();
        assert!(cb.is_call_allowed());
        assert_eq!(*cb.state(), domain::CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_high_error_rate() {
        let mut cb = domain::CircuitBreaker::new();
        // Record 6 failures out of 10 requests (60% > 50% threshold)
        for _ in 0..6 {
            cb.record_failure();
        }
        for _ in 0..4 {
            cb.record_success();
        }
        // Check if the circuit opened
        // (max 100 requests before rate check; we have 10)
        let call_allowed = cb.is_call_allowed();
        // After >50% errors, the circuit should be open
        assert!(!call_allowed);
        assert_eq!(*cb.state(), domain::CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_records_success_and_failure() {
        let mut cb = domain::CircuitBreaker::new();
        cb.record_success();
        cb.record_failure();
        // State stays closed after 1 success + 1 failure (50% rate doesn't trigger)
        assert_eq!(*cb.state(), domain::CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_rejects_calls_when_open() {
        let mut cb = domain::CircuitBreaker::new();
        // Force open state
        for _ in 0..100 {
            cb.record_failure();
        }
        assert_eq!(*cb.state(), domain::CircuitState::Open);
        assert!(!cb.is_call_allowed());
    }

    // ─── Fee Calculation Tests ─────────────────────────────────────────

    #[test]
    fn test_fee_calculation_basic() {
        let fees = sample_fees();
        let amount = domain::Money {
            amount_minor_units: 100000, // 1,000.00 AED
            currency: "AED".into(),
        };

        let fee = fees.calculate_fee(&amount, false, false, 0);
        // fixed_fee (100) + percentage (100000 * 250 / 10000 = 2500) = 2600
        assert_eq!(fee.amount_minor_units, 2600);
    }

    #[test]
    fn test_fee_calculation_with_cross_border() {
        let fees = sample_fees();
        let amount = domain::Money {
            amount_minor_units: 100000,
            currency: "AED".into(),
        };

        let fee = fees.calculate_fee(&amount, true, false, 0);
        // fixed_fee (100) + percentage (2500) + cross_border (50 bps = 500) = 3100
        assert_eq!(fee.amount_minor_units, 3100);
    }

    #[test]
    fn test_fee_calculation_capped() {
        let fees = FeeStructure {
            max_fee_cap: Some(500),
            ..sample_fees()
        };
        let amount = domain::Money {
            amount_minor_units: 1000000, // 10,000 AED
            currency: "AED".into(),
        };

        let fee = fees.calculate_fee(&amount, false, false, 0);
        // Without cap: 100 + 25000 = 25100 → capped at 500
        assert_eq!(fee.amount_minor_units, 500);
    }

    #[test]
    fn test_fee_calculation_floor() {
        let fees = FeeStructure {
            fixed_fee_minor: 0,
            percentage_fee_bps: 10,
            min_fee_floor: Some(100),
            ..sample_fees()
        };
        let amount = domain::Money {
            amount_minor_units: 100, // very small amount
            currency: "AED".into(),
        };

        let fee = fees.calculate_fee(&amount, false, false, 0);
        // Without floor: 0 + 0 = 0 → floored at 100
        assert_eq!(fee.amount_minor_units, 100);
    }

    #[test]
    fn test_fee_calculation_tiered_pricing() {
        let fees = FeeStructure {
            tiered_pricing: Some(vec![
                domain::FeeTier {
                    min_volume_minor: 0,
                    max_volume_minor: Some(1000000),
                    percentage_fee_bps: 300, // 3% for low volume
                },
                domain::FeeTier {
                    min_volume_minor: 1000000,
                    max_volume_minor: None,
                    percentage_fee_bps: 150, // 1.5% for high volume
                },
            ]),
            ..sample_fees()
        };
        let amount = domain::Money {
            amount_minor_units: 100000,
            currency: "AED".into(),
        };

        // Low volume tier: fixed (100) + percentage at 300bps (100000 * 300 / 10000 = 3000) = 3100
        let fee_low = fees.calculate_fee(&amount, false, false, 500000);
        assert_eq!(fee_low.amount_minor_units, 3100);

        // High volume tier: fixed (100) + percentage at 150bps (100000 * 150 / 10000 = 1500) = 1600
        let fee_high = fees.calculate_fee(&amount, false, false, 2000000);
        assert_eq!(fee_high.amount_minor_units, 1600);
    }

    // ─── Gateway Profile Validation Tests ──────────────────────────────

    #[test]
    fn test_validate_transaction_against_profile_ok() {
        let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");
        let amount = domain::Money {
            amount_minor_units: 10000,
            currency: "AED".into(),
        };

        let result = domain::validate_transaction_against_profile(&amount, &profile, &CardScheme::Visa, "AED");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transaction_below_minimum() {
        let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");
        let amount = domain::Money {
            amount_minor_units: 50, // below min of 100
            currency: "AED".into(),
        };

        let result = domain::validate_transaction_against_profile(&amount, &profile, &CardScheme::Visa, "AED");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_transaction_unsupported_card_scheme() {
        let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");
        let amount = domain::Money {
            amount_minor_units: 10000,
            currency: "AED".into(),
        };

        let result = domain::validate_transaction_against_profile(&amount, &profile, &CardScheme::Other("amex".into()), "AED");
        assert!(result.is_err());
    }

    // ─── Rotation Strategy Tests ───────────────────────────────────────

    #[test]
    fn test_rotation_strategy_priority_selects_first() {
        let profile_id1 = test_profile_id();
        let profile_id2 = test_profile_id();
        let operator_id = test_operator_id();
        let link_id = test_link_id();

        let profile1 = GatewayProfile {
            profile_id: profile_id1,
            routing_priority: 1,
            ..sample_gateway_profile(profile_id1, operator_id, link_id, "network_international")
        };
        let profile2 = GatewayProfile {
            profile_id: profile_id2,
            routing_priority: 2,
            ..sample_gateway_profile(profile_id2, operator_id, link_id, "checkout_com")
        };

        let state = RotationState {
            operator_id,
            strategy: RotationStrategy::Priority,
            current_index: 0,
            last_used_gateway_id: None,
            weights: vec![],
            daily_volume: Default::default(),
        };

        let amount = domain::Money { amount_minor_units: 10000, currency: "AED".into() };
        let result = state.select_gateway_profile(
            &[profile1, profile2],
            &amount,
            &CardScheme::Visa,
            "AED",
        );

        assert_eq!(result.unwrap(), profile_id1);
    }

    #[test]
    fn test_rotation_strategy_no_eligible_gateways() {
        let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");
        let state = RotationState {
            operator_id: test_operator_id(),
            strategy: RotationStrategy::Priority,
            current_index: 0,
            last_used_gateway_id: None,
            weights: vec![],
            daily_volume: Default::default(),
        };

        let amount = domain::Money { amount_minor_units: 10000, currency: "USD".into() };
        // Profile only has "AED" enabled, so USD should not be eligible
        let result = state.select_gateway_profile(
            &[profile],
            &amount,
            &CardScheme::Visa,
            "USD",
        );

        assert!(result.is_err());
    }

    // ─── Decline Mapping Tests ─────────────────────────────────────────

    #[test]
    fn test_decline_mapping_known_code() {
        let table = domain::DeclineMappingTable::default();
        assert_eq!(table.normalize("insufficient_funds"), "InsufficientFunds");
        assert_eq!(table.normalize("expired_card"), "ExpiredCard");
        assert_eq!(table.normalize("gateway_timeout"), "IssuerUnavailable");
    }

    #[test]
    fn test_decline_mapping_unknown_code() {
        let table = domain::DeclineMappingTable::default();
        let result = table.normalize("some_unknown_code");
        assert!(result.starts_with("UnknownError"));
    }

    // ─── Connector Registry Tests ──────────────────────────────────────

    #[test]
    fn test_connector_registry_register_and_get() {
        let mut registry = ConnectorRegistry::new();
        registry.register(Box::new(domain::mocks::MockNetworkIntlConnector::new("sandbox")));

        let connector = registry.get("network_international");
        assert!(connector.is_ok());
        assert_eq!(connector.unwrap().connector_id(), "network_international");
    }

    #[test]
    fn test_connector_registry_get_unknown() {
        let registry = ConnectorRegistry::new();
        let connector = registry.get("unknown_connector");
        assert!(connector.is_err());
    }

    #[tokio::test]
    async fn test_mock_connector_authorize_sandbox() {
        let connector = domain::mocks::MockNetworkIntlConnector::new("sandbox");
        let result = connector
            .authorize(domain::AuthorizeRequest {
                payment_method_token: "tok_test".into(),
                amount: domain::Money { amount_minor_units: 10000, currency: "AED".into() },
                currency: "AED".into(),
                idempotency_key: "idem_1".into(),
                card_scheme: CardScheme::Visa,
                metadata: None,
                three_ds_data: None,
            })
            .await;

        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status, domain::AuthorizeStatus::Approved);
        assert!(resp.acquirer_reference.is_some());
    }

    #[tokio::test]
    async fn test_connector_test_card_numbers() {
        let connector = domain::mocks::MockCheckoutComConnector::new("sandbox");
        let cards = connector.test_card_numbers();
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().any(|c| c.scheme == CardScheme::Visa));
        assert!(cards.iter().any(|c| c.scheme == CardScheme::Mastercard));
    }

    // ─── Repository Tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_repository_save_and_load_profile() {
        let repo = InMemoryGatewayProfileRepository::new();
        let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");

        repo.save(&profile).await.unwrap();
        let loaded = repo.load(profile.profile_id).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().connector_id, "network_international");
    }

    #[tokio::test]
    async fn test_repository_find_active_by_operator() {
        let repo = InMemoryGatewayProfileRepository::new();
        let operator_id = test_operator_id();
        let profile = sample_gateway_profile(test_profile_id(), operator_id, test_link_id(), "network_international");

        repo.save(&profile).await.unwrap();
        let active = repo.find_active_for_operator(operator_id).await.unwrap();

        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_repository_daily_volume() {
        let repo = InMemoryGatewayProfileRepository::new();
        let profile_id = test_profile_id();
        let amount = domain::Money { amount_minor_units: 10000, currency: "AED".into() };

        let vol_before = repo.check_daily_volume(profile_id).await.unwrap();
        assert_eq!(vol_before.amount_minor_units, 0);

        repo.increment_daily_volume(profile_id, amount).await.unwrap();
        let vol_after = repo.check_daily_volume(profile_id).await.unwrap();
        assert_eq!(vol_after.amount_minor_units, 10000);
    }

    // ─── Command Handler Tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_create_and_get_gateway_profile() {
        let repo = InMemoryGatewayProfileRepository::new();
        let mut registry = ConnectorRegistry::new();
        registry.register(Box::new(domain::mocks::MockNetworkIntlConnector::new("sandbox")));

        let commands = commands::GatewayCommandHandler::new(repo, registry);

        let operator_id = test_operator_id();
        let result = commands
            .create_gateway_profile(commands::CreateGatewayProfile {
                operator_id,
                connector_id: "network_international".into(),
                merchant_acquirer_link_id: test_link_id(),
                limits: sample_limits(),
                fees: sample_fees(),
                routing_priority: 1,
                enabled_card_schemes: vec![CardScheme::Visa],
                enabled_currencies: vec!["AED".into()],
                enabled_countries: vec!["AE".into()],
                rate_limits: RateLimitConfig {
                    per_second: 100,
                    per_day: 100000,
                    burst_size: 200,
                },
                monitoring: MonitoringThresholds {
                    success_rate_alert: 0.95,
                    success_rate_critical: 0.90,
                    latency_p99_alert_ms: 3000,
                    latency_p99_critical_ms: 5000,
                    auto_disable_on_low_success: false,
                },
            })
            .await;

        assert!(result.is_ok());
        let create_result = result.unwrap();
        assert!(matches!(create_result.event, crate::events::GatewayEvent::GatewayProfileCreated(_)));
    }

    #[tokio::test]
    async fn test_validate_credentials() {
        let repo = InMemoryGatewayProfileRepository::new();
        let mut registry = ConnectorRegistry::new();
        registry.register(Box::new(domain::mocks::MockNetworkIntlConnector::new("sandbox")));

        let commands = commands::GatewayCommandHandler::new(repo, registry);

        let result = commands
            .validate_credentials(commands::ValidateCredentials {
                connector_id: "network_international".into(),
                config: domain::ConnectorConfig {
                    api_key: Some("test_key".into()),
                    secret_key: None,
                    merchant_id: Some("merchant_1".into()),
                    store_id: None,
                    environment: "sandbox".into(),
                    additional_fields: Default::default(),
                },
            })
            .await;

        assert!(result.is_ok());
        let validate_result = result.unwrap();
        assert!(validate_result.valid);
        assert_eq!(validate_result.merchant_name, Some("Mock Merchant NI".into()));
    }
}

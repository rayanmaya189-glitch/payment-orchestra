//! Command handler tests.

use crate::commands::{self, CommandHandler};
use crate::domain::{
    self, CardScheme, ConnectorRegistry, MonitoringThresholds,
    RateLimitConfig,
};
use crate::repository::InMemoryGatewayProfileRepository;

use super::{sample_limits, sample_fees, test_operator_id, test_link_id};

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

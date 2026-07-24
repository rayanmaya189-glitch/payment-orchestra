//! Stripe / mock connector tests: validation, rotation, decline mapping, registry.

use crate::domain::{
    self, CardScheme, ConnectorRegistry, GatewayProfile, MonitoringThresholds,
    ProfileStatus, RateLimitConfig, RotationState, RotationStrategy, TransactionLimits,
};

use super::{sample_gateway_profile, sample_limits, sample_fees, test_profile_id, test_operator_id, test_link_id};

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

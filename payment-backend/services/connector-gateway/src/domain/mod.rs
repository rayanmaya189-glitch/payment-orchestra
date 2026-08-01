//! Connector-gateway domain model.
//! Anti-Corruption Layer — normalizes N acquirer APIs into one protocol.
//! Each domain concept has its own file within this module.

// Sub-modules — one file per concept
pub mod circuit_breaker;
pub mod connector;
pub mod connector_registry;
pub mod error;
pub mod gateway_profile;
pub mod routing;
pub mod types;

// Mock connectors — split per CONVENTIONS.md
pub mod mock_network_intl;
pub mod mock_checkout_com;
pub mod mock_telr;
pub mod mock_connectors;

// Stripe connector — split per CONVENTIONS.md
pub mod stripe_connector;
pub mod stripe_impl;
pub mod stripe_webhook;
pub mod stripe_credential;

// UAE payment gateway connectors
pub mod checkout_com_connector;
pub mod network_intl_connector;
pub mod telr_connector;
pub mod tap_payments_connector;
pub mod paytabs_connector;
pub mod mamo_connector;
pub mod amazon_ps_connector;
pub mod aani_connector;

// Existing sub-modules
pub mod onboarding;

// Re-exports for convenience
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
pub use types::{CardScheme, SettlementCycle, SettlementFormat};
pub use connector_registry::ConnectorRegistry;
pub use error::{ConnectorError, GatewayError};
pub use gateway_profile::{FeeStructure, FeeTier, GatewayProfile, MonitoringThresholds, ProfileStatus, RateLimitConfig, TransactionLimits, validate_transaction_against_profile};
pub use mock_connectors::{MockCheckoutComConnector, MockNetworkIntlConnector, MockTelrConnector};
pub use onboarding::{OnboardingField, OnboardingSchema, SelectOption, FieldType};
pub use routing::{RotationState, RotationStrategy};
pub use stripe_connector::StripeConnector;
pub use checkout_com_connector::CheckoutComConnector;
pub use network_intl_connector::NetworkIntlConnector;
pub use telr_connector::TelrConnector;
pub use tap_payments_connector::TapPaymentsConnector;
pub use paytabs_connector::PayTabsConnector;
pub use mamo_connector::MamoConnector;
pub use amazon_ps_connector::AmazonPsConnector;
pub use aani_connector::AaniConnector;
pub use types::*;

//! Connector-gateway domain model.
//! Anti-Corruption Layer — normalizes N acquirer APIs into one protocol.
//! Each domain concept has its own file within this module.

// Sub-modules — one file per concept
pub mod circuit_breaker;
pub mod connector;
pub mod connector_registry;
pub mod error;
pub mod gateway_profile;
pub mod mock_connectors;
pub mod routing;
pub mod types;

// Existing sub-modules (not extracted)
pub mod onboarding;
pub mod stripe_connector;

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
pub use types::*;

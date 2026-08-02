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

// India payment gateway connectors
pub mod billdesk_connector;
pub mod cashfree_connector;
pub mod payu_connector;
pub mod phonepe_connector;
pub mod razorpay_connector;
pub mod ccavenue_connector;
pub mod paytm_connector;
pub mod juspay_connector;
pub mod instamojo_connector;
pub mod easebuzz_connector;

// India bank-specific connectors
pub mod icici_connector;
pub mod hdfc_connector;
pub mod axis_connector;
pub mod sbi_connector;

// Additional India connectors
pub mod pinelabs_connector;
pub mod worldline_connector;
pub mod zaakpay_connector;
pub mod payglocal_connector;
pub mod kotak_connector;
pub mod yesbank_connector;
pub mod indusind_connector;
pub mod mswipe_connector;
pub mod fampay_connector;
pub mod zestmoney_connector;
pub mod direcpay_connector;
pub mod atom_connector;

// Enterprise connectors
pub mod adyen_connector;
pub mod paypal_connector;

// UPI ecosystem connector
pub mod upi_connector;

// Central FX rate service
pub mod fx_service;

// Network token management
pub mod network_token;

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
pub use billdesk_connector::BilldeskConnector;
pub use cashfree_connector::CashfreeConnector;
pub use payu_connector::PayuConnector;
pub use phonepe_connector::PhonepeConnector;
pub use razorpay_connector::RazorpayConnector;
pub use ccavenue_connector::CcavenueConnector;
pub use paytm_connector::PaytmConnector;
pub use juspay_connector::JuspayConnector;
pub use instamojo_connector::InstamojoConnector;
pub use easebuzz_connector::EasebuzzConnector;
pub use icici_connector::IciciConnector;
pub use hdfc_connector::HdfcConnector;
pub use axis_connector::AxisConnector;
pub use sbi_connector::SbiConnector;
pub use pinelabs_connector::PineLabsConnector;
pub use worldline_connector::WorldlineConnector;
pub use zaakpay_connector::ZaakpayConnector;
pub use payglocal_connector::PayGlocalConnector;
pub use kotak_connector::KotakConnector;
pub use yesbank_connector::YesbankConnector;
pub use indusind_connector::IndusindConnector;
pub use mswipe_connector::MswipeConnector;
pub use fampay_connector::FampayConnector;
pub use zestmoney_connector::ZestmoneyConnector;
pub use direcpay_connector::DirecpayConnector;
pub use atom_connector::AtomConnector;
pub use adyen_connector::AdyenConnector;
pub use paypal_connector::PaypalConnector;
pub use upi_connector::UpiConnector;
pub use fx_service::FxRateService;
pub use network_token::{NetworkToken, NetworkTokenStatus, NetworkType, NetworkTokenRepository};
pub use types::*;

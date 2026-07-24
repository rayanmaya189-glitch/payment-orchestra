//! AcquirerConnector trait implementation for Stripe.
//!
//! Split into modules for CONVENTIONS.md compliance:
//! - payment.rs: authorize, capture, void, refund, status_check
//! - fx.rs: get_fx_rate, settlement_cycle, check_3ds_enrollment, authenticate_3ds
//! - token.rs: provision_network_token, account_updater, poll_settlement

mod payment;
mod fx;
mod token;

use std::collections::HashMap;
use std::time::Instant;

use async_trait::async_trait;

use super::connector::{AcquirerConnector, ConnectorCapabilities};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::stripe_connector::StripeConnector;
use super::types::*;

#[async_trait]
impl AcquirerConnector for StripeConnector {
    fn connector_id(&self) -> &str {
        "stripe"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: true,
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supports_fx_conversion: true,
            supported_card_schemes: vec![
                CardScheme::Visa,
                CardScheme::Mastercard,
                CardScheme::Amex,
                CardScheme::Other("Discover".into()),
                CardScheme::Other("JCB".into()),
                CardScheme::Other("Diners".into()),
                CardScheme::Other("UnionPay".into()),
            ],
            supported_currencies: vec![
                "AED".into(), "USD".into(), "EUR".into(), "GBP".into(),
                "AUD".into(), "CAD".into(), "SGD".into(), "HKD".into(),
                "CHF".into(), "JPY".into(), "INR".into(), "SAR".into(),
            ],
            settlement_format: SettlementFormat::Webhook,
            settlement_cycle: SettlementCycle::NextDay,
            cross_border_fee_bps: 100,
        }
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "stripe".into(),
            fields: vec![
                OnboardingField {
                    name: "secret_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "Secret Key".into(),
                    validation_regex: Some(r"^(sk|rk)_(test|live)_[a-zA-Z0-9]+$".into()),
                    help_text: Some("Find in Stripe Dashboard > Developers > API Keys".into()),
                },
                OnboardingField {
                    name: "webhook_secret".into(),
                    field_type: FieldType::Password,
                    required: false,
                    label: "Webhook Signing Secret".into(),
                    validation_regex: Some(r"^whsec_[a-zA-Z0-9]+$".into()),
                    help_text: Some("Find in Stripe Dashboard > Developers > Webhooks > Your Endpoint > Signing Secret".into()),
                },
                OnboardingField {
                    name: "environment".into(),
                    field_type: FieldType::Select {
                        options: vec![
                            SelectOption { value: "sandbox".into(), label: "Sandbox (Test)".into() },
                            SelectOption { value: "production".into(), label: "Production (Live)".into() },
                        ],
                    },
                    required: true,
                    label: "Environment".into(),
                    validation_regex: None,
                    help_text: None,
                },
            ],
        }
    }

    // Payment operations — delegated to payment.rs
    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        self.authorize_impl(req).await
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        self.capture_impl(req).await
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        self.void_impl(req).await
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        self.refund_impl(req).await
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        self.status_check_impl(req).await
    }

    // FX & 3DS — delegated to fx.rs
    async fn get_fx_rate(&self, req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        self.get_fx_rate_impl(req).await
    }

    fn settlement_cycle(&self) -> SettlementCycle {
        SettlementCycle::NextDay
    }

    async fn check_3ds_enrollment(&self, req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
        self.check_3ds_enrollment_impl(req).await
    }

    async fn authenticate_3ds(&self, req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
        self.authenticate_3ds_impl(req)
    }

    // Token operations — delegated to token.rs
    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
        self.provision_network_token_impl(req).await
    }

    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation(
            "Account updater queries not directly supported via Stripe API. Use Stripe's built-in automatic card updater.".into(),
        ))
    }

    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        Ok(vec![])
    }

    // Delegated to stripe_webhook.rs and stripe_credential.rs
    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        StripeConnector::verify_webhook_signature(self, headers, body)
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        StripeConnector::parse_webhook(self, body)
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        StripeConnector::validate_credentials(self, config).await
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        StripeConnector::test_connection(self, config).await
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        StripeConnector::test_card_numbers(self)
    }
}

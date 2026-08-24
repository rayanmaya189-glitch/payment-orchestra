//! Mock Checkout.com connector for testing (International PSP).

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use ring::digest::{Context, SHA256};

use super::connector::{AcquirerConnector, ConnectorCapabilities};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut ctx = Context::new(&SHA256);
    ctx.update(data);
    ctx.finish().as_ref().to_vec()
}

pub struct MockCheckoutComConnector {
    #[allow(dead_code)]
    environment: String,
}

impl MockCheckoutComConnector {
    pub fn new(environment: &str) -> Self {
        Self { environment: environment.to_string() }
    }
}

#[async_trait]
impl AcquirerConnector for MockCheckoutComConnector {
    fn connector_id(&self) -> &str {
        "checkout_com"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: true,
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supports_fx_conversion: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex],
            supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()],
            settlement_format: SettlementFormat::Webhook,
            settlement_cycle: SettlementCycle::NextDay,
            cross_border_fee_bps: 75,
        }
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "checkout_com".into(),
            fields: vec![
                OnboardingField {
                    name: "secret_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "Secret Key".into(),
                    validation_regex: Some(r"^sk_(test|live)_[a-zA-Z0-9]+$".into()),
                    help_text: Some("Find in Dashboard > Settings > API Keys".into()),
                },
                OnboardingField {
                    name: "environment".into(),
                    field_type: FieldType::Select {
                        options: vec![
                            SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                            SelectOption { value: "production".into(), label: "Production".into() },
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

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        Ok(AuthorizeResponse {
            status: AuthorizeStatus::Approved,
            acquirer_reference: Some(format!("cko_auth_{}", uuid::Uuid::now_v7())),
            decline_reason: None,
            approved_amount: Some(req.amount),
            three_ds_data: None,
            latency_ms: 95,
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        Ok(CaptureResponse {
            success: true,
            acquirer_reference: Some(format!("cko_cap_{}", uuid::Uuid::now_v7())),
            amount_captured: req.amount,
            latency_ms: 65,
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        Ok(VoidResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference),
            latency_ms: 45,
        })
    }

    async fn refund(&self, _req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        Ok(RefundResponse {
            success: true,
            acquirer_reference: Some(format!("cko_ref_{}", uuid::Uuid::now_v7())),
            refund_id: Some(format!("cko_rf_{}", uuid::Uuid::now_v7())),
            latency_ms: 70,
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        Ok(StatusCheckResponse {
            status: AuthorizeStatus::Approved,
            acquirer_reference: Some(req.acquirer_reference),
            amount: None,
            latency_ms: 40,
        })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        Ok(FxRateResponse {
            rate: "3.6725".into(),
            rate_minor_units: 3672500,
            converted_amount: Money { amount_minor_units: 10000, currency: "AED".into() },
            fee: Some(Money { amount_minor_units: 25, currency: "AED".into() }),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        })
    }

    fn settlement_cycle(&self) -> SettlementCycle {
        SettlementCycle::NextDay
    }

    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
        Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None })
    }

    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("3DS not implemented".into()))
    }

    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
        Ok(ProvisionTokenResponse {
            network_token: format!("cko_tok_{}", uuid::Uuid::now_v7()),
            token_expiry_month: req.expiry_month,
            token_expiry_year: req.expiry_year + 1,
            cryptogram: Some("AEBB0C4C7A4D3B".into()),
        })
    }

    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
        Ok(AccountUpdateResult {
            updated: true,
            new_expiry_month: Some(12),
            new_expiry_year: Some(2028),
        })
    }

    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        Ok(vec![])
    }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let expected = format!("sha256={}", hex::encode(sha256(body)));
        if headers.get("x-signature").map(|s| s == &expected).unwrap_or(false) {
            Ok(())
        } else {
            Err(ConnectorError::InvalidSignature)
        }
    }

    fn parse_webhook(&self, _body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        Ok(ConnectorEvent {
            event_type: "payment.captured".into(),
            payload: serde_json::json!({"status": "captured"}),
        })
    }

    async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        Ok(CredentialValidationResult {
            valid: true,
            merchant_name: Some("Mock Merchant CKO".into()),
            permissions: vec!["authorize".into(), "capture".into()],
            error_message: None,
        })
    }

    async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        Ok(ConnectionTestResult {
            success: true,
            merchant_name: Some("Mock Merchant CKO".into()),
            latency_ms: 130,
            error_message: None,
        })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![
            TestCardNumber {
                label: "Visa Approved".into(),
                card_number: "4242424242424242".into(),
                scheme: CardScheme::Visa,
                scenario: "authorize_approved".into(),
            },
            TestCardNumber {
                label: "Mastercard 3DS".into(),
                card_number: "5200000000000007".into(),
                scheme: CardScheme::Mastercard,
                scenario: "requires_3ds".into(),
            },
        ]
    }
}

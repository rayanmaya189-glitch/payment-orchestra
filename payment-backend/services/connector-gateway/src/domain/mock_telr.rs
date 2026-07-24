//! Mock Telr connector for testing (Regional PSP).

use std::collections::HashMap;

use async_trait::async_trait;

use super::connector::{AcquirerConnector, ConnectorCapabilities};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct MockTelrConnector {
    #[allow(dead_code)]
    environment: String,
}

impl MockTelrConnector {
    pub fn new(environment: &str) -> Self {
        Self { environment: environment.to_string() }
    }
}

#[async_trait]
impl AcquirerConnector for MockTelrConnector {
    fn connector_id(&self) -> &str {
        "telr"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: false,
            supports_partial_refund: true,
            supports_native_idempotency_key: false,
            supports_webhook_settlement: false,
            supports_realtime_status_check: true,
            supports_fx_conversion: false,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec!["AED".into(), "USD".into()],
            settlement_format: SettlementFormat::PollingApi,
            settlement_cycle: SettlementCycle::ThreeDays,
            cross_border_fee_bps: 100,
        }
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "telr".into(),
            fields: vec![
                OnboardingField {
                    name: "store_id".into(),
                    field_type: FieldType::String,
                    required: true,
                    label: "Store ID".into(),
                    validation_regex: None,
                    help_text: Some("Provided by Telr at onboarding".into()),
                },
                OnboardingField {
                    name: "api_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "API Key".into(),
                    validation_regex: None,
                    help_text: None,
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
            acquirer_reference: Some(format!("telr_auth_{}", uuid::Uuid::now_v7())),
            decline_reason: None,
            approved_amount: Some(req.amount),
            three_ds_data: None,
            latency_ms: 200,
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        Ok(CaptureResponse {
            success: true,
            acquirer_reference: Some(format!("telr_cap_{}", uuid::Uuid::now_v7())),
            amount_captured: req.amount,
            latency_ms: 150,
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        Ok(VoidResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference),
            latency_ms: 100,
        })
    }

    async fn refund(&self, _req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        Ok(RefundResponse {
            success: true,
            acquirer_reference: Some(format!("telr_ref_{}", uuid::Uuid::now_v7())),
            refund_id: Some(format!("telr_rf_{}", uuid::Uuid::now_v7())),
            latency_ms: 120,
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        Ok(StatusCheckResponse {
            status: AuthorizeStatus::Approved,
            acquirer_reference: Some(req.acquirer_reference),
            amount: None,
            latency_ms: 80,
        })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("FX not supported by Telr".into()))
    }

    fn settlement_cycle(&self) -> SettlementCycle {
        SettlementCycle::ThreeDays
    }

    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
        Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None })
    }

    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("3DS not implemented".into()))
    }

    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("Network tokens not supported".into()))
    }

    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("Account updater not supported".into()))
    }

    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        Ok(vec![])
    }

    fn verify_webhook_signature(&self, _headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("Webhooks not supported by Telr".into()))
    }

    fn parse_webhook(&self, _body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("Webhooks not supported by Telr".into()))
    }

    async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        Ok(CredentialValidationResult {
            valid: true,
            merchant_name: Some("Mock Merchant Telr".into()),
            permissions: vec!["authorize".into(), "capture".into(), "refund".into()],
            error_message: None,
        })
    }

    async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        Ok(ConnectionTestResult {
            success: true,
            merchant_name: Some("Mock Merchant Telr".into()),
            latency_ms: 180,
            error_message: None,
        })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![
            TestCardNumber {
                label: "Visa Approved".into(),
                card_number: "4111111111111111".into(),
                scheme: CardScheme::Visa,
                scenario: "authorize_approved".into(),
            },
            TestCardNumber {
                label: "Mastercard Declined".into(),
                card_number: "5100000000000008".into(),
                scheme: CardScheme::Mastercard,
                scenario: "authorize_declined".into(),
            },
        ]
    }
}

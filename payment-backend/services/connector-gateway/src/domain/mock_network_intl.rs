//! Mock Network International connector for testing (UAE Regional Acquirer).

use std::collections::HashMap;

use async_trait::async_trait;

use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct MockNetworkIntlConnector {
    circuit_breaker: std::sync::Mutex<CircuitBreaker>,
    #[allow(dead_code)]
    decline_table: DeclineMappingTable,
    #[allow(dead_code)]
    environment: String,
}

impl MockNetworkIntlConnector {
    pub fn new(environment: &str) -> Self {
        Self {
            circuit_breaker: std::sync::Mutex::new(CircuitBreaker::new()),
            decline_table: DeclineMappingTable::default(),
            environment: environment.to_string(),
        }
    }
}

#[async_trait]
impl AcquirerConnector for MockNetworkIntlConnector {
    fn connector_id(&self) -> &str {
        "network_international"
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: false,
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supports_fx_conversion: false,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec!["AED".into()],
            settlement_format: SettlementFormat::Webhook,
            settlement_cycle: SettlementCycle::SameDay,
            cross_border_fee_bps: 50,
        }
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "network_international".into(),
            fields: vec![
                OnboardingField {
                    name: "api_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "API Key".into(),
                    validation_regex: None,
                    help_text: None,
                },
                OnboardingField {
                    name: "merchant_id".into(),
                    field_type: FieldType::String,
                    required: true,
                    label: "Merchant ID".into(),
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
        {
            let mut cb = self.circuit_breaker.lock().unwrap();
            if !cb.is_call_allowed() {
                return Err(ConnectorError::CircuitBreakerOpen("network_international".into()));
            }
        }
        if self.environment == "sandbox" {
            self.circuit_breaker.lock().unwrap().record_success();
            Ok(AuthorizeResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(format!("ni_auth_{}", uuid::Uuid::now_v7())),
                decline_reason: None,
                approved_amount: Some(req.amount),
                three_ds_data: None,
                latency_ms: 120,
            })
        } else {
            self.circuit_breaker.lock().unwrap().record_failure();
            Err(ConnectorError::AuthenticationFailed("production not configured in mock".into()))
        }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        Ok(CaptureResponse {
            success: true,
            acquirer_reference: Some(format!("ni_cap_{}", uuid::Uuid::now_v7())),
            amount_captured: req.amount,
            latency_ms: 80,
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        Ok(VoidResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference),
            latency_ms: 60,
        })
    }

    async fn refund(&self, _req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        Ok(RefundResponse {
            success: true,
            acquirer_reference: Some(format!("ni_ref_{}", uuid::Uuid::now_v7())),
            refund_id: Some(format!("ni_rf_{}", uuid::Uuid::now_v7())),
            latency_ms: 90,
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        Ok(StatusCheckResponse {
            status: AuthorizeStatus::Approved,
            acquirer_reference: Some(req.acquirer_reference),
            amount: None,
            latency_ms: 50,
        })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        Err(ConnectorError::UnsupportedOperation("FX not supported by Network International".into()))
    }

    fn settlement_cycle(&self) -> SettlementCycle {
        SettlementCycle::SameDay
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

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
        if headers.get("x-signature").map(|s| s == "valid_sig").unwrap_or(false) {
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
            merchant_name: Some("Mock Merchant NI".into()),
            permissions: vec!["authorize".into(), "capture".into(), "refund".into()],
            error_message: None,
        })
    }

    async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        Ok(ConnectionTestResult {
            success: true,
            merchant_name: Some("Mock Merchant NI".into()),
            latency_ms: 150,
            error_message: None,
        })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![TestCardNumber {
            label: "Visa Approved".into(),
            card_number: "4111111111111111".into(),
            scheme: CardScheme::Visa,
            scenario: "authorize_approved".into(),
        }]
    }
}

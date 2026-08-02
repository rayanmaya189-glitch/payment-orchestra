//! Sandbox mode — mock connectors for testing without real PSP credentials.
//!
//! Provides:
//! - Pre-configured sandbox connectors with deterministic responses
//! - Test card numbers with configurable outcomes
//! - Simulated latency and failure rates
//! - Sandbox webhook delivery

use std::collections::HashMap;
use async_trait::async_trait;
use super::connector::{AcquirerConnector, ConnectorCapabilities};
use super::error::ConnectorError;
use super::types::*;

/// Sandbox connector that returns deterministic responses.
pub struct SandboxConnector {
    test_cards: HashMap<String, TestCardOutcome>,
    simulated_latency_ms: u32,
    simulated_failure_rate: f64,
}

/// Outcome for a test card number.
#[derive(Debug, Clone)]
pub struct TestCardOutcome {
    pub status: AuthorizeStatus,
    pub decline_reason: Option<String>,
    pub requires_3ds: bool,
}

impl Default for SandboxConnector {
    fn default() -> Self {
        let mut test_cards = HashMap::new();

        // Visa success
        test_cards.insert(
            "4242424242424242".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Approved,
                decline_reason: None,
                requires_3ds: false,
            },
        );

        // Visa declined
        test_cards.insert(
            "4000000000000002".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Declined,
                decline_reason: Some("DoNotHonor".to_string()),
                requires_3ds: false,
            },
        );

        // Mastercard success
        test_cards.insert(
            "5555555555554444".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Approved,
                decline_reason: None,
                requires_3ds: false,
            },
        );

        // 3DS required
        test_cards.insert(
            "4000000000003220".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Requires3DS,
                decline_reason: None,
                requires_3ds: true,
            },
        );

        // Amex success
        test_cards.insert(
            "378282246310005".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Approved,
                decline_reason: None,
                requires_3ds: false,
            },
        );

        // Insufficient funds
        test_cards.insert(
            "4000000000009995".to_string(),
            TestCardOutcome {
                status: AuthorizeStatus::Declined,
                decline_reason: Some("InsufficientFunds".to_string()),
                requires_3ds: false,
            },
        );

        Self {
            test_cards,
            simulated_latency_ms: 100,
            simulated_failure_rate: 0.0,
        }
    }
}

impl SandboxConnector {
    /// Create a new sandbox connector with custom settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set simulated latency for all operations.
    pub fn with_latency(mut self, ms: u32) -> Self {
        self.simulated_latency_ms = ms;
        self
    }

    /// Set simulated failure rate (0.0 to 1.0).
    pub fn with_failure_rate(mut self, rate: f64) -> Self {
        self.simulated_failure_rate = rate;
        self
    }

    /// Add a custom test card outcome.
    pub fn with_test_card(mut self, card_number: String, outcome: TestCardOutcome) -> Self {
        self.test_cards.insert(card_number, outcome);
        self
    }

    fn should_fail(&self) -> bool {
        if self.simulated_failure_rate <= 0.0 {
            return false;
        }
        let random: f64 = rand::random();
        random < self.simulated_failure_rate
    }
}

#[async_trait]
impl AcquirerConnector for SandboxConnector {
    fn connector_id(&self) -> &str {
        "sandbox"
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
            ],
            supported_currencies: vec![
                "USD".into(),
                "EUR".into(),
                "GBP".into(),
                "AED".into(),
                "INR".into(),
            ],
            settlement_format: SettlementFormat::Webhook,
            settlement_cycle: SettlementCycle::NextDay,
            cross_border_fee_bps: 0,
        }
    }

    fn onboarding_schema(&self) -> super::onboarding::OnboardingSchema {
        super::onboarding::OnboardingSchema {
            connector_id: "sandbox".into(),
            fields: vec![super::onboarding::OnboardingField {
                name: "api_key".into(),
                field_type: super::onboarding::FieldType::String,
                required: false,
                label: "Sandbox API Key (optional)".into(),
                validation_regex: None,
                help_text: Some("Sandbox mode doesn't require real credentials".into()),
            }],
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        // Simulate latency
        tokio::time::sleep(std::time::Duration::from_millis(self.simulated_latency_ms as u64)).await;

        // Check for simulated failure
        if self.should_fail() {
            return Err(ConnectorError::NetworkError("Simulated network failure".into()));
        }

        // Look up test card outcome
        let card_number = req.payment_method_token.replace(' ', "");
        let outcome = self.test_cards.get(&card_number).cloned().unwrap_or(
            // Default: approve unknown cards
            TestCardOutcome {
                status: AuthorizeStatus::Approved,
                decline_reason: None,
                requires_3ds: false,
            },
        );

        Ok(AuthorizeResponse {
            status: outcome.status.clone(),
            acquirer_reference: Some(format!("sb_{}", Uuid::now_v7())),
            decline_reason: outcome.decline_reason,
            approved_amount: if outcome.status == AuthorizeStatus::Approved {
                Some(req.amount.clone())
            } else {
                None
            },
            three_ds_data: if outcome.requires_3ds {
                Some(ThreeDsData {
                    three_ds_version: "2.1.0".into(),
                    acs_url: Some("https://sandbox.3ds-acs.com/authorize".into()),
                    pareq: Some("sandbox_pareq".into()),
                    md: Some("sandbox_md".into()),
                    session_data: Some("sandbox_session".into()),
                })
            } else {
                None
            },
            latency_ms: self.simulated_latency_ms,
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        tokio::time::sleep(std::time::Duration::from_millis(self.simulated_latency_ms as u64)).await;

        if self.should_fail() {
            return Err(ConnectorError::NetworkError("Simulated capture failure".into()));
        }

        Ok(CaptureResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference),
            amount_captured: Money {
                amount_minor_units: req.amount.amount_minor_units,
                currency: req.amount.currency,
            },
            latency_ms: self.simulated_latency_ms,
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        tokio::time::sleep(std::time::Duration::from_millis(self.simulated_latency_ms as u64)).await;

        Ok(VoidResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference),
            latency_ms: self.simulated_latency_ms,
        })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        tokio::time::sleep(std::time::Duration::from_millis(self.simulated_latency_ms as u64)).await;

        if self.should_fail() {
            return Err(ConnectorError::NetworkError("Simulated refund failure".into()));
        }

        Ok(RefundResponse {
            success: true,
            acquirer_reference: Some(req.acquirer_reference.clone()),
            refund_id: Some(format!("sb_ref_{}", Uuid::now_v7())),
            latency_ms: self.simulated_latency_ms,
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        tokio::time::sleep(std::time::Duration::from_millis(self.simulated_latency_ms as u64)).await;

        Ok(StatusCheckResponse {
            status: AuthorizeStatus::Approved,
            acquirer_reference: Some(req.acquirer_reference),
            amount: None,
            latency_ms: self.simulated_latency_ms,
        })
    }

    async fn get_fx_rate(&self, req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
        Ok(FxRateResponse {
            rate: "1.0".into(),
            rate_minor_units: 1000000,
            converted_amount: Money {
                amount_minor_units: req.amount.amount_minor_units,
                currency: req.target_currency.clone(),
            },
            fee: None,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
        })
    }

    fn settlement_cycle(&self) -> SettlementCycle {
        SettlementCycle::NextDay
    }

    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
        Ok(Check3dsResponse {
            requires_3ds: false,
            three_ds_data: None,
        })
    }

    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
        Ok(Authenticate3dsResponse {
            authenticated: true,
            three_ds_status: "authenticated".into(),
            eci: Some("05".into()),
        })
    }

    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
        Ok(ProvisionTokenResponse {
            network_token: format!("tok_sandbox_{}", Uuid::now_v7()),
            token_expiry_month: req.expiry_month,
            token_expiry_year: req.expiry_year,
            cryptogram: Some("sandbox_cryptogram".into()),
        })
    }

    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
        Ok(AccountUpdateResult {
            updated: false,
            new_expiry_month: None,
            new_expiry_year: None,
        })
    }

    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        Ok(vec![])
    }

    fn verify_webhook_signature(&self, _headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
        Ok(()) // Sandbox accepts all signatures
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let payload: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid JSON: {}", e)))?;
        Ok(ConnectorEvent {
            event_type: payload["event_type"].as_str().unwrap_or("unknown").to_string(),
            payload,
        })
    }

    async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        Ok(CredentialValidationResult {
            valid: true,
            merchant_name: Some("Sandbox Merchant".into()),
            permissions: vec!["authorize".into(), "capture".into(), "refund".into()],
            error_message: None,
        })
    }

    async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        Ok(ConnectionTestResult {
            success: true,
            merchant_name: Some("Sandbox".into()),
            latency_ms: self.simulated_latency_ms,
            error_message: None,
        })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![
            TestCardNumber {
                label: "Visa — Success".into(),
                card_number: "4242424242424242".into(),
                scheme: CardScheme::Visa,
                scenario: "authorize_approved".into(),
            },
            TestCardNumber {
                label: "Visa — Declined".into(),
                card_number: "4000000000000002".into(),
                scheme: CardScheme::Visa,
                scenario: "authorize_declined".into(),
            },
            TestCardNumber {
                label: "Mastercard — Success".into(),
                card_number: "5555555555554444".into(),
                scheme: CardScheme::Mastercard,
                scenario: "authorize_approved".into(),
            },
            TestCardNumber {
                label: "3DS Required".into(),
                card_number: "4000000000003220".into(),
                scheme: CardScheme::Visa,
                scenario: "requires_3ds".into(),
            },
            TestCardNumber {
                label: "Amex — Success".into(),
                card_number: "378282246310005".into(),
                scheme: CardScheme::Amex,
                scenario: "authorize_approved".into(),
            },
            TestCardNumber {
                label: "Insufficient Funds".into(),
                card_number: "4000000000009995".into(),
                scheme: CardScheme::Visa,
                scenario: "decline_insufficient_funds".into(),
            },
        ]
    }
}

use uuid::Uuid;

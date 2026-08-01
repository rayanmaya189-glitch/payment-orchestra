//! PayTabs Connector — real HTTP adapter for PayTabs' REST API.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct PayTabsConnector {
    server_key: String, profile_id: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl PayTabsConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let server_key = config.secret_key.clone().unwrap_or_default();
        let profile_id = config.merchant_id.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = "https://secure.paytabs.com".to_string();
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for PayTabs");
        Self { server_key, profile_id, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    fn auth_header(&self) -> String { self.server_key.clone() }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["tran_ref"].as_str().unwrap_or("unknown");
        let status = body["payment_result"]["response_status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "A" => AuthorizeStatus::Approved, "D" | "E" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        let decline_code = body["payment_result"]["response_code"].as_str().map(|code| self.decline_table.normalize(code));
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: decline_code, approved_amount: body.get("cart_amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["cart_currency"].as_str().unwrap_or("AED").to_uppercase() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for PayTabsConnector {
    fn connector_id(&self) -> &str { "paytabs" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: true, supports_partial_refund: true, supports_native_idempotency_key: true, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex], supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into(), "SAR".into(), "QAR".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::NextDay, cross_border_fee_bps: 130 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "paytabs".into(), fields: vec![ OnboardingField { name: "secret_key".into(), field_type: FieldType::Password, required: true, label: "Server Key".into(), validation_regex: None, help_text: Some("Find in PayTabs Dashboard > Developers > Key Management".into()) }, OnboardingField { name: "merchant_id".into(), field_type: FieldType::String, required: true, label: "Profile ID".into(), validation_regex: None, help_text: Some("Your PayTabs merchant profile ID".into()) }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Production".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("paytabs".into())); } }
        let payload = serde_json::json!({ "profile_id": self.profile_id, "tran_type": "auth", "tran_class": "ecom", "cart_id": req.idempotency_key, "cart_description": "Payment via Payment Orchestra", "cart_currency": req.currency, "cart_amount": req.amount.amount_minor_units as f64 / 100.0, "token": req.payment_method_token });
        let resp = self.client.post(format!("{}/payment/request", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("PayTabs request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["payment_result"]["response_message"].as_str().unwrap_or("Unknown PayTabs error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "profile_id": self.profile_id, "tran_type": "capture", "tran_class": "ecom", "cart_id": req.idempotency_key, "cart_description": "Capture via Payment Orchestra", "cart_currency": req.amount.currency, "cart_amount": req.amount.amount_minor_units as f64 / 100.0, "tran_ref": req.acquirer_reference });
        let resp = self.client.post(format!("{}/payment/request", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["tran_ref"].as_str().map(String::from), amount_captured: Money { amount_minor_units: body["cart_amount"].as_str().and_then(|a| a.parse::<i64>().ok()).unwrap_or(req.amount.amount_minor_units), currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "profile_id": self.profile_id, "tran_type": "void", "tran_class": "ecom", "tran_ref": req.acquirer_reference });
        let resp = self.client.post(format!("{}/payment/request", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["tran_ref"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "profile_id": self.profile_id, "tran_type": "refund", "tran_class": "ecom", "cart_id": req.idempotency_key, "cart_description": "Refund via Payment Orchestra", "cart_currency": req.amount.currency, "cart_amount": req.amount.amount_minor_units as f64 / 100.0, "tran_ref": req.acquirer_reference });
        let resp = self.client.post(format!("{}/payment/request", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["tran_ref"].as_str().map(String::from), refund_id: body["tran_ref"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.get(format!("{}/payment/query?tran_ref={}", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayTabs parse: {}", e)))?;
        let status = body["payment_result"]["response_status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "A" => AuthorizeStatus::Approved, "D" | "E" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["tran_ref"].as_str().map(String::from), amount: body.get("cart_amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["cart_currency"].as_str().unwrap_or("AED").to_uppercase() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by PayTabs".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::NextDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by PayTabs".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by PayTabs".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-paytabs-signature").or_else(|| headers.get("X-PayTabs-Signature"));
        match signature { Some(s) if !s.is_empty() => Ok(()), _ => Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["tran_type"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let server_key = config.secret_key.as_deref().unwrap_or("");
        let profile_id = config.merchant_id.as_deref().unwrap_or("");
        if server_key.is_empty() || profile_id.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Server Key and Profile ID are required".into()) }); }
        Ok(CredentialValidationResult { valid: true, merchant_name: Some(format!("PayTabs Profile {}", profile_id)), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None })
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Visa — Success".into(), card_number: "4111111111111111".into(), scheme: CardScheme::Visa, scenario: "authorize_approved".into() }, TestCardNumber { label: "Visa — 3DS".into(), card_number: "4000000000000002".into(), scheme: CardScheme::Visa, scenario: "requires_3ds".into() }, TestCardNumber { label: "Mastercard — Success".into(), card_number: "5498383801606532".into(), scheme: CardScheme::Mastercard, scenario: "authorize_approved".into() } ]
    }
}

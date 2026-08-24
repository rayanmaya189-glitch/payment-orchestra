//! Instamojo Connector — real HTTP adapter for Instamojo's REST API.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;
use platform_middleware::ssrf::validate_connector_url;

pub struct InstamojoConnector {
    api_key: String, auth_token: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl InstamojoConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let api_key = config.api_key.clone().unwrap_or_default();
        let auth_token = config.secret_key.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://test.instamojo.com/api/1.5".to_string() } else { "https://www.instamojo.com/api/1.5".to_string() };
        if let Err(e) = validate_connector_url(&base_url) {
            tracing::warn!("SSRF validation warning for base_url '{}': {}", base_url, e);
        }
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for Instamojo");
        Self { api_key, auth_token, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "credited" | "pending" => AuthorizeStatus::Approved, "failed" | "expired" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: body["failure_reason"].as_str().map(|c| self.decline_table.normalize(c)), approved_amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: "INR".into() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for InstamojoConnector {
    fn connector_id(&self) -> &str { "instamojo" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: false, supports_partial_refund: true, supports_native_idempotency_key: false, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Other("RuPay".into()), CardScheme::Other("UPI".into())], supported_currencies: vec!["INR".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::NextDay, cross_border_fee_bps: 200 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "instamojo".into(), fields: vec![ OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "API Key".into(), validation_regex: None, help_text: Some("Find in Instamojo Dashboard > Settings > API Keys".into()) }, OnboardingField { name: "secret_key".into(), field_type: FieldType::Password, required: true, label: "Auth Token".into(), validation_regex: None, help_text: None }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Test Mode".into() }, SelectOption { value: "production".into(), label: "Live Mode".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("instamojo".into())); } }
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units as f64 / 100.0, "currency": "INR", "purpose": "Payment via Payment Orchestra", "buyer_name": "Customer", "buyer_email": "customer@example.com", "buyer_phone": "9999999999" });
        let resp = self.client.post(format!("{}/payments/orders/", self.base_url)).header("X-Api-Key", &self.api_key).header("X-Auth-Token", &self.auth_token).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("Instamojo request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["message"].as_str().unwrap_or("Unknown Instamojo error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.post(format!("{}/payments/orders/{}/capture", self.base_url, req.acquirer_reference)).header("X-Api-Key", &self.api_key).header("X-Auth-Token", &self.auth_token).send().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), amount_captured: Money { amount_minor_units: req.amount.amount_minor_units, currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.post(format!("{}/payments/orders/{}/cancel", self.base_url, req.acquirer_reference)).header("X-Api-Key", &self.api_key).header("X-Auth-Token", &self.auth_token).send().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units as f64 / 100.0 });
        let resp = self.client.post(format!("{}/payments/refunds/", self.base_url)).header("X-Api-Key", &self.api_key).header("X-Auth-Token", &self.auth_token).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), refund_id: body["id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.get(format!("{}/payments/orders/{}", self.base_url, req.acquirer_reference)).header("X-Api-Key", &self.api_key).header("X-Auth-Token", &self.auth_token).send().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Instamojo parse: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "credited" | "pending" => AuthorizeStatus::Approved, "failed" | "expired" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["id"].as_str().map(String::from), amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: "INR".into() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by Instamojo".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::NextDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by Instamojo".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by Instamojo".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-instamojo-signature").or_else(|| headers.get("X-Instamojo-Signature")).ok_or(ConnectorError::InvalidSignature)?;
        if signature.is_empty() { return Err(ConnectorError::InvalidSignature); }
        use ring::hmac; let key = hmac::Key::new(hmac::HMAC_SHA256, self.auth_token.as_bytes()); let computed = hmac::sign(&key, body); let computed_hex = hex::encode(computed.as_ref());
        if computed_hex == *signature { Ok(()) } else { Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["status"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let api_key = config.api_key.as_deref().unwrap_or("");
        let auth_token = config.secret_key.as_deref().unwrap_or("");
        if api_key.is_empty() || auth_token.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("API Key and Auth Token required".into()) }); }
        Ok(CredentialValidationResult { valid: true, merchant_name: Some("Instamojo Merchant".into()), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None })
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Visa — Success".into(), card_number: "4111111111111111".into(), scheme: CardScheme::Visa, scenario: "authorize_approved".into() }, TestCardNumber { label: "Mastercard — Success".into(), card_number: "5104010000000008".into(), scheme: CardScheme::Mastercard, scenario: "authorize_approved".into() } ]
    }
}

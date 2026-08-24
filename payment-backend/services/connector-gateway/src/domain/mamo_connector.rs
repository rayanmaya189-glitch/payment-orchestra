//! Mamo Connector — real HTTP adapter for Mamo's REST API.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct MamoConnector {
    api_key: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl MamoConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let api_key = config.api_key.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://sandbox.mamopay.com/api/v1".to_string() } else { "https://api.mamopay.com/v1".to_string() };
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for Mamo");
        Self { api_key, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    fn auth_header(&self) -> String { format!("Bearer {}", self.api_key) }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "authorized" | "captured" | "succeeded" => AuthorizeStatus::Approved, "declined" | "failed" | "cancelled" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        let decline_code = body["decline_code"].as_str().map(|code| self.decline_table.normalize(code));
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: decline_code, approved_amount: body.get("amount").and_then(|a| a.as_i64()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for MamoConnector {
    fn connector_id(&self) -> &str { "mamo" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: false, supports_partial_refund: true, supports_native_idempotency_key: true, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex], supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::NextDay, cross_border_fee_bps: 140 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "mamo".into(), fields: vec![ OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "API Key".into(), validation_regex: None, help_text: Some("Find in Mamo Dashboard > Settings > API Keys".into()) }, OnboardingField { name: "webhook_secret".into(), field_type: FieldType::Password, required: false, label: "Webhook Secret".into(), validation_regex: None, help_text: None }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Production".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("mamo".into())); } }
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units, "currency": req.currency, "payment_method": req.payment_method_token, "capture": false });
        let mut request = self.client.post(format!("{}/payments", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json");
        if !req.idempotency_key.is_empty() { request = request.header("Idempotency-Key", &req.idempotency_key); }
        let resp = request.json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("Mamo request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["message"].as_str().unwrap_or("Unknown Mamo error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units });
        let resp = self.client.post(format!("{}/payments/{}/capture", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), amount_captured: Money { amount_minor_units: body["amount"].as_i64().unwrap_or(req.amount.amount_minor_units), currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.post(format!("{}/payments/{}/cancel", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").send().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units });
        let resp = self.client.post(format!("{}/payments/{}/refunds", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), refund_id: body["id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.get(format!("{}/payments/{}", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).send().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "authorized" | "captured" | "succeeded" => AuthorizeStatus::Approved, "declined" | "failed" | "cancelled" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["id"].as_str().map(String::from), amount: body.get("amount").and_then(|a| a.as_i64()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by Mamo".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::NextDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by Mamo".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by Mamo".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-mamo-signature").or_else(|| headers.get("X-Mamo-Signature")).ok_or(ConnectorError::InvalidSignature)?;
        if signature.is_empty() { return Err(ConnectorError::InvalidSignature); }
        use ring::hmac; let key = hmac::Key::new(hmac::HMAC_SHA256, self.api_key.as_bytes()); let computed = hmac::sign(&key, body); let computed_hex = hex::encode(computed.as_ref());
        if computed_hex == *signature { Ok(()) } else { Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["type"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let api_key = config.api_key.as_deref().unwrap_or("");
        let resp = self.client.get(format!("{}/merchants/me", self.base_url)).header("Authorization", format!("Bearer {}", api_key)).send().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo validate: {}", e)))?;
        if !resp.status().is_success() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Invalid API key".into()) }); }
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Mamo parse: {}", e)))?;
        Ok(CredentialValidationResult { valid: true, merchant_name: body["name"].as_str().map(String::from), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None })
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Visa — Success".into(), card_number: "4242424242424242".into(), scheme: CardScheme::Visa, scenario: "authorize_approved".into() }, TestCardNumber { label: "Mastercard — Success".into(), card_number: "5555555555554444".into(), scheme: CardScheme::Mastercard, scenario: "authorize_approved".into() } ]
    }
}

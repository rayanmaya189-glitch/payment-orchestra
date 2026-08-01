//! Aani Connector — real HTTP adapter for Aani (AANI) instant payments.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct AaniConnector {
    client_id: String, client_secret: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl AaniConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let client_id = config.api_key.clone().unwrap_or_default();
        let client_secret = config.secret_key.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://sandbox.aani.ae/api/v1".to_string() } else { "https://api.aani.ae/v1".to_string() };
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for Aani");
        Self { client_id, client_secret, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("account_blocked".into(), "DoNotHonor".into()), ("account_not_found".into(), "InvalidCard".into()), ("amount_exceeded".into(), "TransactionLimitExceeded".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    async fn get_access_token(&self) -> Result<String, ConnectorError> {
        let resp = self.client.post(format!("{}/oauth/token", self.base_url)).header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("grant_type", "client_credentials"), ("client_id", &self.client_id), ("client_secret", &self.client_secret)])
            .send().await.map_err(|e| ConnectorError::NetworkError(format!("Aani token: {}", e)))?;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Aani parse: {}", e)))?;
        body["access_token"].as_str().map(String::from).ok_or_else(|| ConnectorError::AuthenticationFailed("Failed to get Aani access token".into()))
    }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["payment_id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "COMPLETED" | "SETTLED" | "ACCEPTED" => AuthorizeStatus::Approved, "REJECTED" | "FAILED" | "CANCELLED" => AuthorizeStatus::Declined, "PENDING" => AuthorizeStatus::Approved, _ => AuthorizeStatus::Declined };
        let decline_code = body["rejection_reason"].as_str().map(|code| self.decline_table.normalize(code));
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: decline_code, approved_amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for AaniConnector {
    fn connector_id(&self) -> &str { "aani" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: false, supports_partial_refund: true, supports_native_idempotency_key: true, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![], supported_currencies: vec!["AED".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::SameDay, cross_border_fee_bps: 0 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "aani".into(), fields: vec![ OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "Client ID".into(), validation_regex: None, help_text: Some("Your Aani API client ID".into()) }, OnboardingField { name: "secret_key".into(), field_type: FieldType::Password, required: true, label: "Client Secret".into(), validation_regex: None, help_text: Some("Your Aani API client secret".into()) }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Production".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("aani".into())); } }
        let token = self.get_access_token().await?;
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units.to_string(), "currency": req.currency, "payer_account": req.payment_method_token, "reference": req.idempotency_key, "description": "Payment via Payment Orchestra" });
        let mut request = self.client.post(format!("{}/payments/initiate", self.base_url)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json");
        if !req.idempotency_key.is_empty() { request = request.header("Idempotency-Key", &req.idempotency_key); }
        let resp = request.json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("Aani request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Aani parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["error"]["message"].as_str().unwrap_or("Unknown Aani error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        Ok(CaptureResponse { success: true, acquirer_reference: Some(req.acquirer_reference), amount_captured: req.amount, latency_ms: 0 })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let resp = self.client.post(format!("{}/payments/{}/cancel", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").send().await.map_err(|e| ConnectorError::NetworkError(format!("Aani void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Aani parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["payment_id"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let payload = serde_json::json!({ "amount": req.amount.amount_minor_units.to_string(), "currency": req.amount.currency, "reason": "Refund via Payment Orchestra" });
        let resp = self.client.post(format!("{}/payments/{}/refund", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Aani refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Aani parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["refund_id"].as_str().map(String::from), refund_id: body["refund_id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let resp = self.client.get(format!("{}/payments/{}", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).send().await.map_err(|e| ConnectorError::NetworkError(format!("Aani status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Aani parse: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "COMPLETED" | "SETTLED" | "ACCEPTED" => AuthorizeStatus::Approved, "REJECTED" | "FAILED" | "CANCELLED" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["payment_id"].as_str().map(String::from), amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by Aani — domestic AED only".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::SameDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("3DS not applicable for Aani — account-to-account payments".into())) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not applicable for Aani".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not applicable for Aani".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-aani-signature").or_else(|| headers.get("X-Aani-Signature")).ok_or(ConnectorError::InvalidSignature)?;
        if signature.is_empty() { return Err(ConnectorError::InvalidSignature); }
        use ring::hmac; let key = hmac::Key::new(hmac::HMAC_SHA256, self.client_secret.as_bytes()); let computed = hmac::sign(&key, body); let computed_hex = hex::encode(computed.as_ref());
        if computed_hex == *signature { Ok(()) } else { Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["event_type"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let client_id = config.api_key.as_deref().unwrap_or("");
        let client_secret = config.secret_key.as_deref().unwrap_or("");
        if client_id.is_empty() || client_secret.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Client ID and Client Secret are required".into()) }); }
        match self.get_access_token().await { Ok(_) => Ok(CredentialValidationResult { valid: true, merchant_name: Some("Aani Instant Payments".into()), permissions: vec!["authorize".into(), "refund".into()], error_message: None }), Err(e) => Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some(format!("Credential validation failed: {}", e)) }) }
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Aani Test Account — Success".into(), card_number: "1234567890123456".into(), scheme: CardScheme::Other("Aani".into()), scenario: "authorize_approved".into() } ]
    }
}

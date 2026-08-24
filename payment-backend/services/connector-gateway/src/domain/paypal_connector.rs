//! PayPal India Connector — real HTTP adapter for PayPal's REST API.
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

pub struct PaypalConnector {
    client_id: String, client_secret: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
    cached_token: Mutex<Option<(String, std::time::Instant)>>,
}

impl PaypalConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let client_id = config.api_key.clone().unwrap_or_default();
        let client_secret = config.secret_key.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://api-m.sandbox.paypal.com".to_string() } else { "https://api-m.paypal.com".to_string() };
        if let Err(e) = validate_connector_url(&base_url) {
            tracing::warn!("SSRF validation warning for base_url '{}': {}", base_url, e);
        }
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for PayPal");
        Self { client_id, client_secret, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])), cached_token: Mutex::new(None) }
    }

    async fn get_access_token(&self) -> Result<String, ConnectorError> {
        {
            let cache = self.cached_token.lock().map_err(|e| ConnectorError::NetworkError(format!("Token cache lock: {}", e)))?;
            if let Some((token, expires_at)) = cache.as_ref() {
                if std::time::Instant::now() < *expires_at {
                    return Ok(token.clone());
                }
            }
        }
        use base64::Engine;
        let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret).as_bytes());
        let resp = self.client.post(format!("{}/v1/oauth2/token", self.base_url)).header("Authorization", format!("Basic {}", credentials)).header("Content-Type", "application/x-www-form-urlencoded").form(&[("grant_type", "client_credentials")]).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal token: {}", e)))?;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal parse: {}", e)))?;
        let token = body["access_token"].as_str().map(String::from).ok_or_else(|| ConnectorError::AuthenticationFailed("Failed to get PayPal access token".into()))?;
        {
            let mut cache = self.cached_token.lock().map_err(|e| ConnectorError::NetworkError(format!("Token cache lock: {}", e)))?;
            *cache = Some((token.clone(), std::time::Instant::now() + std::time::Duration::from_secs(3600)));
        }
        Ok(token)
    }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "COMPLETED" | "APPROVED" | "CAPTURED" => AuthorizeStatus::Approved, "DECLINED" | "FAILED" | "CANCELED" => AuthorizeStatus::Declined, "PENDING" | "PROCESSING" => AuthorizeStatus::Approved, _ => AuthorizeStatus::Declined };
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: None, approved_amount: body.get("purchase_units").and_then(|u| u.get(0)).and_then(|u| u.get("amount")).and_then(|a| a.get("value")).and_then(|v| v.as_str()).and_then(|v| v.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: "INR".into() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for PaypalConnector {
    fn connector_id(&self) -> &str { "paypal" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: true, supports_partial_refund: true, supports_native_idempotency_key: true, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: true, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex, CardScheme::Other("RuPay".into())], supported_currencies: vec!["INR".into(), "USD".into(), "EUR".into(), "GBP".into(), "AED".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::NextDay, cross_border_fee_bps: 250 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "paypal".into(), fields: vec![ OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "Client ID".into(), validation_regex: None, help_text: Some("Find in PayPal Developer Dashboard".into()) }, OnboardingField { name: "secret_key".into(), field_type: FieldType::Password, required: true, label: "Client Secret".into(), validation_regex: None, help_text: None }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Live".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("paypal".into())); } }
        let token = self.get_access_token().await?;
        let payload = serde_json::json!({ "intent": "AUTHORIZE", "purchase_units": [{ "reference_id": req.idempotency_key, "amount": { "currency_code": req.currency, "value": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0) } }], "payment_source": { "card": { "number": req.payment_method_token } } });
        let resp = self.client.post(format!("{}/v2/checkout/orders", self.base_url)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("PayPal request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["message"].as_str().unwrap_or("Unknown PayPal error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let resp = self.client.post(format!("{}/v2/checkout/orders/{}/capture", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").send().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), amount_captured: Money { amount_minor_units: req.amount.amount_minor_units, currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let resp = self.client.post(format!("{}/v2/checkout/orders/{}/cancel", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").send().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: Some(req.acquirer_reference), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let payload = serde_json::json!({ "amount": { "currency_code": req.amount.currency, "value": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0) } });
        let resp = self.client.post(format!("{}/v2/payments/captures/{}/refund", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["id"].as_str().map(String::from), refund_id: body["id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let token = self.get_access_token().await?;
        let resp = self.client.get(format!("{}/v2/checkout/orders/{}", self.base_url, req.acquirer_reference)).header("Authorization", format!("Bearer {}", token)).send().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("PayPal parse: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "COMPLETED" | "APPROVED" | "CAPTURED" => AuthorizeStatus::Approved, "DECLINED" | "FAILED" | "CANCELED" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["id"].as_str().map(String::from), amount: body.get("purchase_units").and_then(|u| u.get(0)).and_then(|u| u.get("amount")).and_then(|a| a.get("value")).and_then(|v| v.as_str()).and_then(|v| v.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: "INR".into() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX via PayPal not yet implemented".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::NextDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by PayPal".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by PayPal".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("paypal-transmission-id").ok_or(ConnectorError::InvalidSignature)?;
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
        if client_id.is_empty() || client_secret.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Client ID and Secret required".into()) }); }
        match self.get_access_token().await { Ok(_) => Ok(CredentialValidationResult { valid: true, merchant_name: Some("PayPal Merchant".into()), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None }), Err(e) => Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some(format!("Validation failed: {}", e)) }) }
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

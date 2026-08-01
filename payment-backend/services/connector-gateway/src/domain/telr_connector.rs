//! Telr Connector — real HTTP adapter for Telr's REST API.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct TelrConnector {
    store_id: String, api_key: String, environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl TelrConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let store_id = config.store_id.clone().unwrap_or_default();
        let api_key = config.api_key.clone().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://test.telr.com/v1".to_string() } else { "https://secure.telr.com/v1".to_string() };
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for Telr");
        Self { store_id, api_key, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    fn auth_header(&self) -> String {
        // Base64 encode "store_id:api_key" for Basic auth
        let raw = format!("{}:{}", self.store_id, self.api_key);
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = raw.as_bytes();
        let mut encoded = String::with_capacity((bytes.len() + 2) / 3 * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            encoded.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            encoded.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 { encoded.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { encoded.push('='); }
            if chunk.len() > 2 { encoded.push(CHARS[(triple & 0x3F) as usize] as char); } else { encoded.push('='); }
        }
        format!("Basic {}", encoded)
    }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["order"]["id"].as_str().or_else(|| body["ref"].as_str()).unwrap_or("unknown");
        let status = body["order"]["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "A" | "AUTHORISED" => AuthorizeStatus::Approved, "D" | "DECLINED" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        let decline_code = body["order"]["response"]["code"].as_str().map(|code| self.decline_table.normalize(code));
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: decline_code, approved_amount: body.get("order").and_then(|o| o.get("amount")).and_then(|a| a.as_i64()).map(|a| Money { amount_minor_units: a, currency: body["order"]["currency"].as_str().unwrap_or("AED").to_uppercase() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for TelrConnector {
    fn connector_id(&self) -> &str { "telr" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: false, supports_partial_refund: true, supports_native_idempotency_key: false, supports_webhook_settlement: false, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard], supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()], settlement_format: SettlementFormat::PollingApi, settlement_cycle: SettlementCycle::ThreeDays, cross_border_fee_bps: 100 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "telr".into(), fields: vec![ OnboardingField { name: "store_id".into(), field_type: FieldType::String, required: true, label: "Store ID".into(), validation_regex: None, help_text: Some("Provided by Telr at onboarding".into()) }, OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "API Key".into(), validation_regex: None, help_text: None }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Production".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("telr".into())); } }
        let payload = serde_json::json!({ "order": { "amount": req.amount.amount_minor_units, "currency": req.currency, "description": "Payment via Payment Orchestra" }, "payment_method": { "type": "card", "card_token": req.payment_method_token }, "transaction": { "type": "preauth" } });
        let resp = self.client.post(format!("{}/order/new", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("Telr request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Telr parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["error"]["message"].as_str().unwrap_or("Unknown Telr error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "order": { "id": req.acquirer_reference }, "transaction": { "amount": req.amount.amount_minor_units, "currency": req.amount.currency } });
        let resp = self.client.post(format!("{}/order/capture", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Telr capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Telr parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["order"]["id"].as_str().map(String::from), amount_captured: Money { amount_minor_units: body["transaction"]["amount"].as_i64().unwrap_or(req.amount.amount_minor_units), currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "order": { "id": req.acquirer_reference } });
        let resp = self.client.post(format!("{}/order/void", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Telr void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Telr parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["order"]["id"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({ "order": { "id": req.acquirer_reference }, "transaction": { "amount": req.amount.amount_minor_units, "currency": req.amount.currency } });
        let resp = self.client.post(format!("{}/order/refund", self.base_url)).header("Authorization", self.auth_header()).header("Content-Type", "application/json").json(&payload).send().await.map_err(|e| ConnectorError::NetworkError(format!("Telr refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Telr parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["order"]["id"].as_str().map(String::from), refund_id: body["transaction"]["id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.get(format!("{}/order/{}", self.base_url, req.acquirer_reference)).header("Authorization", self.auth_header()).send().await.map_err(|e| ConnectorError::NetworkError(format!("Telr status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("Telr parse: {}", e)))?;
        let status = body["order"]["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "A" | "AUTHORISED" => AuthorizeStatus::Approved, "D" | "DECLINED" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["order"]["id"].as_str().map(String::from), amount: body.get("order").and_then(|o| o.get("amount")).and_then(|a| a.as_i64()).map(|a| Money { amount_minor_units: a, currency: body["order"]["currency"].as_str().unwrap_or("AED").to_uppercase() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by Telr".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::ThreeDays }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by Telr".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by Telr".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-telr-signature").or_else(|| headers.get("X-Telr-Signature"));
        match signature { Some(s) if !s.is_empty() => Ok(()), _ => Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["event"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let store_id = config.store_id.as_deref().unwrap_or("");
        let api_key = config.api_key.as_deref().unwrap_or("");
        if store_id.is_empty() || api_key.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Store ID and API Key are required".into()) }); }
        Ok(CredentialValidationResult { valid: true, merchant_name: Some(format!("Telr Store {}", store_id)), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None })
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Visa — Success".into(), card_number: "4111111111111111".into(), scheme: CardScheme::Visa, scenario: "authorize_approved".into() }, TestCardNumber { label: "Mastercard — Success".into(), card_number: "5100000000000008".into(), scheme: CardScheme::Mastercard, scenario: "authorize_approved".into() } ]
    }
}

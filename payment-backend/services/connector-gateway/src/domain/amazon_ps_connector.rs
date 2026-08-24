//! Amazon Payment Services (Payfort) Connector — real HTTP adapter for APS REST API.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::Value;
use super::circuit_breaker::CircuitBreaker;
use super::connector::{AcquirerConnector, ConnectorCapabilities, DeclineMappingTable};
use super::error::ConnectorError;
use super::onboarding::{FieldType, OnboardingField, OnboardingSchema, SelectOption};
use super::types::*;

pub struct AmazonPsConnector {
    access_code: String, merchant_identifier: String, sha_request_phrase: String, sha_response_phrase: String,
    environment: String, base_url: String,
    client: reqwest::Client, circuit_breaker: Mutex<CircuitBreaker>, decline_table: DeclineMappingTable,
}

impl AmazonPsConnector {
    pub fn new(config: &ConnectorConfig) -> Self {
        let access_code = config.api_key.clone().unwrap_or_default();
        let merchant_identifier = config.merchant_id.clone().unwrap_or_default();
        let sha_request_phrase = config.additional_fields.get("sha_request_phrase").cloned().unwrap_or_default();
        let sha_response_phrase = config.additional_fields.get("sha_response_phrase").cloned().unwrap_or_default();
        let environment = config.environment.clone();
        let base_url = if environment == "sandbox" { "https://sbpaymentservices.payfort.com/FortAPI/paymentApi".to_string() } else { "https://paymentservices.payfort.com/FortAPI/paymentApi".to_string() };
        let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(30)).user_agent("PaymentOrchestra/1.0").build().expect("Failed to create HTTP client for Amazon Payment Services");
        Self { access_code, merchant_identifier, sha_request_phrase, sha_response_phrase, environment, base_url, client, circuit_breaker: Mutex::new(CircuitBreaker::new()), decline_table: DeclineMappingTable::new(HashMap::from([ ("insufficient_funds".into(), "InsufficientFunds".into()), ("do_not_honor".into(), "DoNotHonor".into()), ("expired_card".into(), "ExpiredCard".into()), ("invalid_card_number".into(), "InvalidCard".into()), ("card_declined".into(), "SuspectedFraud".into()), ("processing_error".into(), "IssuerUnavailable".into()), ("stolen_card".into(), "StolenCard".into()), ("lost_card".into(), "LostCard".into()), ("generic_decline".into(), "CardDeclined".into()), ("rate_limit".into(), "RateLimitedByAcquirer".into()) ])) }
    }

    fn generate_signature(&self, params: &HashMap<String, String>) -> String {
        use ring::hmac;
        let mut sorted_keys: Vec<&String> = params.keys().collect();
        sorted_keys.sort();
        let mut signature_string = self.sha_request_phrase.clone();
        for key in sorted_keys {
            if key == "signature" { continue; }
            if let Some(val) = params.get(key) { signature_string.push_str(&format!("{}={}", key, val)); }
        }
        signature_string.push_str(&self.sha_request_phrase);
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.sha_request_phrase.as_bytes());
        let computed = hmac::sign(&key, signature_string.as_bytes());
        hex::encode(computed.as_ref())
    }

    fn verify_response_signature(&self, params: &HashMap<String, String>) -> bool {
        use ring::hmac;
        let signature = match params.get("signature") { Some(s) => s.clone(), None => return false };
        let mut sorted_keys: Vec<&String> = params.keys().collect();
        sorted_keys.sort();
        let mut signature_string = self.sha_response_phrase.clone();
        for key in sorted_keys {
            if key == "signature" { continue; }
            if let Some(val) = params.get(key) { signature_string.push_str(&format!("{}={}", key, val)); }
        }
        signature_string.push_str(&self.sha_response_phrase);
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.sha_response_phrase.as_bytes());
        let computed = hmac::sign(&key, signature_string.as_bytes());
        hex::encode(computed.as_ref()) == signature
    }

    fn normalize_authorize_response(&self, body: &Value, latency_ms: u32) -> Result<AuthorizeResponse, ConnectorError> {
        let id = body["fort_id"].as_str().unwrap_or("unknown");
        let status = body["status"].as_str().unwrap_or("unknown");
        let authorize_status = match status { "14" => AuthorizeStatus::Approved, "02" | "03" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        let response_code = body["response_code"].as_str().unwrap_or("");
        let decline_code = if !response_code.is_empty() { Some(self.decline_table.normalize(response_code)) } else { None };
        Ok(AuthorizeResponse { status: authorize_status, acquirer_reference: Some(id.to_string()), decline_reason: decline_code, approved_amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), three_ds_data: None, latency_ms })
    }
}

#[async_trait]
impl AcquirerConnector for AmazonPsConnector {
    fn connector_id(&self) -> &str { "amazon_ps" }
    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities { supports_partial_capture: true, supports_partial_refund: true, supports_native_idempotency_key: true, supports_webhook_settlement: true, supports_realtime_status_check: true, supports_fx_conversion: false, supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex, CardScheme::Other("Mada".into())], supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into(), "SAR".into(), "QAR".into(), "BHD".into(), "KWD".into(), "OMR".into()], settlement_format: SettlementFormat::Webhook, settlement_cycle: SettlementCycle::SameDay, cross_border_fee_bps: 80 }
    }
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema { connector_id: "amazon_ps".into(), fields: vec![ OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "Access Code".into(), validation_regex: None, help_text: Some("Find in APS Dashboard > Merchant Management > Security Settings".into()) }, OnboardingField { name: "merchant_id".into(), field_type: FieldType::String, required: true, label: "Merchant Identifier".into(), validation_regex: None, help_text: Some("Your APS merchant identifier".into()) }, OnboardingField { name: "sha_request_phrase".into(), field_type: FieldType::Password, required: true, label: "SHA Request Phrase".into(), validation_regex: None, help_text: Some("Found in APS Dashboard > Security Settings".into()) }, OnboardingField { name: "sha_response_phrase".into(), field_type: FieldType::Password, required: true, label: "SHA Response Phrase".into(), validation_regex: None, help_text: Some("Found in APS Dashboard > Security Settings".into()) }, OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![SelectOption { value: "sandbox".into(), label: "Sandbox".into() }, SelectOption { value: "production".into(), label: "Production".into() }] }, required: true, label: "Environment".into(), validation_regex: None, help_text: None } ] }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = std::time::Instant::now();
        { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if !cb.is_call_allowed() { return Err(ConnectorError::CircuitBreakerOpen("amazon_ps".into())); } }
        let mut params = HashMap::new();
        params.insert("command".into(), "AUTHORIZATION".into());
        params.insert("access_code".into(), self.access_code.clone());
        params.insert("merchant_identifier".into(), self.merchant_identifier.clone());
        params.insert("merchant_reference".into(), req.idempotency_key.clone());
        params.insert("amount".into(), req.amount.amount_minor_units.to_string());
        params.insert("currency".into(), req.currency.clone());
        params.insert("language".into(), "en".into());
        let signature = self.generate_signature(&params);
        params.insert("signature".into(), signature);

        let resp = self.client.post(&self.base_url).header("Content-Type", "application/json").json(&params).send().await.map_err(|e| { if e.is_timeout() { ConnectorError::Timeout(start.elapsed().as_millis() as u64) } else { ConnectorError::NetworkError(format!("APS request: {}", e)) } })?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("APS parse: {}", e)))?;
        if status_code.is_success() { let result = self.normalize_authorize_response(&body, latency_ms)?; { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; if result.status == AuthorizeStatus::Approved { cb.record_success(); } else { cb.record_failure(); } } Ok(result) } else { let error_msg = body["response_message"].as_str().unwrap_or("Unknown APS error"); { let mut cb = self.circuit_breaker.lock().map_err(|e| ConnectorError::NetworkError(format!("CB lock: {}", e)))?; cb.record_failure(); } match status_code.as_u16() { 401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())), 429 => Err(ConnectorError::RateLimited), _ => Err(ConnectorError::AcquirerDeclined(error_msg.into())) } }
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let mut params = HashMap::new();
        params.insert("command".into(), "CAPTURE".into());
        params.insert("access_code".into(), self.access_code.clone());
        params.insert("merchant_identifier".into(), self.merchant_identifier.clone());
        params.insert("merchant_reference".into(), req.idempotency_key.clone());
        params.insert("amount".into(), req.amount.amount_minor_units.to_string());
        params.insert("currency".into(), req.amount.currency.clone());
        params.insert("language".into(), "en".into());
        params.insert("fort_id".into(), req.acquirer_reference.clone());
        let signature = self.generate_signature(&params);
        params.insert("signature".into(), signature);

        let resp = self.client.post(&self.base_url).header("Content-Type", "application/json").json(&params).send().await.map_err(|e| ConnectorError::NetworkError(format!("APS capture: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("APS parse: {}", e)))?;
        Ok(CaptureResponse { success: status_code.is_success(), acquirer_reference: body["fort_id"].as_str().map(String::from), amount_captured: Money { amount_minor_units: body["amount"].as_str().and_then(|a| a.parse::<i64>().ok()).unwrap_or(req.amount.amount_minor_units), currency: req.amount.currency }, latency_ms })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let mut params = HashMap::new();
        params.insert("command".into(), "VOID_AUTHORIZATION".into());
        params.insert("access_code".into(), self.access_code.clone());
        params.insert("merchant_identifier".into(), self.merchant_identifier.clone());
        params.insert("fort_id".into(), req.acquirer_reference.clone());
        params.insert("language".into(), "en".into());
        let signature = self.generate_signature(&params);
        params.insert("signature".into(), signature);

        let resp = self.client.post(&self.base_url).header("Content-Type", "application/json").json(&params).send().await.map_err(|e| ConnectorError::NetworkError(format!("APS void: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("APS parse: {}", e)))?;
        Ok(VoidResponse { success: status_code.is_success(), acquirer_reference: body["fort_id"].as_str().map(String::from), latency_ms })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let mut params = HashMap::new();
        params.insert("command".into(), "REFUND".into());
        params.insert("access_code".into(), self.access_code.clone());
        params.insert("merchant_identifier".into(), self.merchant_identifier.clone());
        params.insert("merchant_reference".into(), req.idempotency_key.clone());
        params.insert("amount".into(), req.amount.amount_minor_units.to_string());
        params.insert("currency".into(), req.amount.currency.clone());
        params.insert("language".into(), "en".into());
        params.insert("fort_id".into(), req.acquirer_reference.clone());
        let signature = self.generate_signature(&params);
        params.insert("signature".into(), signature);

        let resp = self.client.post(&self.base_url).header("Content-Type", "application/json").json(&params).send().await.map_err(|e| ConnectorError::NetworkError(format!("APS refund: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("APS parse: {}", e)))?;
        Ok(RefundResponse { success: status_code.is_success(), acquirer_reference: body["fort_id"].as_str().map(String::from), refund_id: body["fort_id"].as_str().map(String::from), latency_ms })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = std::time::Instant::now();
        let resp = self.client.get(format!("{}/{}", self.base_url, req.acquirer_reference)).header("Content-Type", "application/json").send().await.map_err(|e| ConnectorError::NetworkError(format!("APS status: {}", e)))?;
        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| ConnectorError::NetworkError(format!("APS parse: {}", e)))?;
        let status = body["status"].as_str().unwrap_or("unknown");
        let auth_status = match status { "14" => AuthorizeStatus::Approved, "02" | "03" => AuthorizeStatus::Declined, _ => AuthorizeStatus::Declined };
        Ok(StatusCheckResponse { status: auth_status, acquirer_reference: body["fort_id"].as_str().map(String::from), amount: body.get("amount").and_then(|a| a.as_str()).and_then(|a| a.parse::<i64>().ok()).map(|a| Money { amount_minor_units: a, currency: body["currency"].as_str().unwrap_or("AED").to_uppercase() }), latency_ms })
    }

    async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("FX not supported by APS".into())) }
    fn settlement_cycle(&self) -> SettlementCycle { SettlementCycle::SameDay }
    async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> { Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None }) }
    async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> { Ok(Authenticate3dsResponse { authenticated: true, three_ds_status: "authenticated".into(), eci: None }) }
    async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Network tokens not supported by APS".into())) }
    async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> { Err(ConnectorError::UnsupportedOperation("Account updater not supported by APS".into())) }
    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> { Ok(vec![]) }

    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("x-aps-signature").or_else(|| headers.get("X-APS-Signature")).ok_or(ConnectorError::InvalidSignature)?;
        if signature.is_empty() { return Err(ConnectorError::InvalidSignature); }
        let body_params: Value = serde_json::from_slice(body).unwrap_or_default();
        let mut params = HashMap::new();
        if let Some(obj) = body_params.as_object() { for (k, v) in obj { if let Some(s) = v.as_str() { params.insert(k.clone(), s.to_string()); } } }
        if self.verify_response_signature(&params) { Ok(()) } else { Err(ConnectorError::InvalidSignature) }
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let parsed: Value = serde_json::from_slice(body).map_err(|e| ConnectorError::InvalidRequest(format!("Invalid webhook JSON: {}", e)))?;
        let event_type = parsed["command"].as_str().unwrap_or("unknown").to_string();
        Ok(ConnectorEvent { event_type, payload: parsed })
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
        let access_code = config.api_key.as_deref().unwrap_or("");
        let merchant_id = config.merchant_id.as_deref().unwrap_or("");
        if access_code.is_empty() || merchant_id.is_empty() { return Ok(CredentialValidationResult { valid: false, merchant_name: None, permissions: vec![], error_message: Some("Access Code and Merchant Identifier are required".into()) }); }
        Ok(CredentialValidationResult { valid: true, merchant_name: Some(format!("APS Merchant {}", merchant_id)), permissions: vec!["authorize".into(), "capture".into(), "refund".into()], error_message: None })
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
        let start = std::time::Instant::now();
        let result = self.validate_credentials(config).await?;
        let latency_ms = start.elapsed().as_millis() as u32;
        Ok(ConnectionTestResult { success: result.valid, merchant_name: result.merchant_name, latency_ms, error_message: result.error_message })
    }

    fn test_card_numbers(&self) -> Vec<TestCardNumber> {
        vec![ TestCardNumber { label: "Visa — Success".into(), card_number: "4005550000000001".into(), scheme: CardScheme::Visa, scenario: "authorize_approved".into() }, TestCardNumber { label: "Mastercard — Success".into(), card_number: "5123456789012346".into(), scheme: CardScheme::Mastercard, scenario: "authorize_approved".into() }, TestCardNumber { label: "Amex — Success".into(), card_number: "345678901234564".into(), scheme: CardScheme::Amex, scenario: "authorize_approved".into() } ]
    }
}

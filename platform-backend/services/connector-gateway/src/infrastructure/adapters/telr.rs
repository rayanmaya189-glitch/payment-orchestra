//! Telr connector implementation.
//!
//! Supports Visa/Mastercard, AED/USD. Polling-based settlement.

use async_trait::async_trait;

use crate::domain::aggregates::AcquirerConnector;
use crate::domain::circuit_breaker::CircuitBreaker;
use crate::domain::value_objects::*;
use shared_types::{CurrencyCode, Money};

pub struct TelrConnector {
    connector_id: String,
    base_url: String,
    store_id: String,
    api_key: String,
    circuit_breaker: CircuitBreaker,
    http_client: reqwest::Client,
}

impl TelrConnector {
    pub fn new(connector_id: String, base_url: String, store_id: String, api_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");
        Self { connector_id, base_url, store_id, api_key, circuit_breaker: CircuitBreaker::new(), http_client }
    }
}

#[async_trait]
impl AcquirerConnector for TelrConnector {
    fn connector_id(&self) -> &str { &self.connector_id }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        if req.card_token.is_empty() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::InvalidRequest("card_token is required".into())); }
        if req.amount.amount_minor_units <= 0 { self.circuit_breaker.record_failure().await; return Err(ConnectorError::InvalidRequest("amount must be positive".into())); }

        let body = serde_json::json!({
            "order": { "id": req.merchant_reference },
            "card": { "token": req.card_token },
            "amount": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0),
            "currency": req.amount.currency.0,
            "capture": false,
        });

        let response = match self.http_client.post(format!("{}/v2/order/prepare", self.base_url))
            .basic_auth(&self.store_id, Some(&self.api_key))
            .header("Content-Type", "application/json")
            .json(&body).send().await
        {
            Ok(r) => r,
            Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Telr API error").to_string())); }
        self.circuit_breaker.record_success().await;

        let telr_status = resp["order"]["status"].as_str().unwrap_or("Unknown");
        let mapped = match telr_status { "Authorised" | "Captured" => "approved", "Declined" => "declined", _ => "error" };

        Ok(AuthorizeResponse {
            status: mapped.to_string(),
            acquirer_reference: resp["order"]["id"].as_str().unwrap_or("").to_string(),
            decline_reason: if mapped == "declined" { resp["order"]["response"]["message"].as_str().map(|s| s.to_string()) } else { None },
            fee: None,
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        let body = if let Some(amount) = &req.amount { serde_json::json!({ "amount": format!("{:.2}", amount.amount_minor_units as f64 / 100.0) }) } else { serde_json::json!({}) };
        let response = match self.http_client.post(format!("{}/v2/order/capture/{}", self.base_url, req.acquirer_reference))
            .basic_auth(&self.store_id, Some(&self.api_key)).header("Content-Type", "application/json").json(&body).send().await
        {
            Ok(r) => r, Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Capture failed: {e}"))); }
        };
        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Capture failed").to_string())); }
        self.circuit_breaker.record_success().await;
        let cap_amount = req.amount.clone().unwrap_or(Money { amount_minor_units: 0, currency: CurrencyCode::new("AED").unwrap() });
        Ok(CaptureResponse { status: resp["order"]["status"].as_str().unwrap_or("Captured").to_string(), captured_amount: cap_amount })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        let response = match self.http_client.post(format!("{}/v2/order/void/{}", self.base_url, req.acquirer_reference))
            .basic_auth(&self.store_id, Some(&self.api_key)).header("Content-Type", "application/json").json(&serde_json::json!({})).send().await
        {
            Ok(r) => r, Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Void failed: {e}"))); }
        };
        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Void failed").to_string())); }
        self.circuit_breaker.record_success().await;
        Ok(VoidResponse { status: resp["order"]["status"].as_str().unwrap_or("Voided").to_string() })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        let body = serde_json::json!({ "amount": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0), "currency": req.amount.currency.0 });
        let response = match self.http_client.post(format!("{}/v2/order/refund/{}", self.base_url, req.acquirer_reference))
            .basic_auth(&self.store_id, Some(&self.api_key)).header("Content-Type", "application/json").json(&body).send().await
        {
            Ok(r) => r, Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Refund failed: {e}"))); }
        };
        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Refund failed").to_string())); }
        self.circuit_breaker.record_success().await;
        Ok(RefundResponse { status: resp["order"]["status"].as_str().unwrap_or("Refunded").to_string(), refund_reference: resp["order"]["id"].as_str().unwrap_or("").to_string() })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        let response = match self.http_client.get(format!("{}/v2/order/{}", self.base_url, req.acquirer_reference))
            .basic_auth(&self.store_id, Some(&self.api_key)).send().await
        {
            Ok(r) => r, Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Status check failed: {e}"))); }
        };
        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Status check failed").to_string())); }
        self.circuit_breaker.record_success().await;
        let mapped = match resp["order"]["status"].as_str().unwrap_or("Unknown") { "Authorised" => "authorized", "Captured" => "captured", "Voided" => "voided", "Refunded" => "refunded", "Declined" => "declined", _ => "unknown" };
        Ok(StatusCheckResponse { status: mapped.to_string(), acquirer_reference: req.acquirer_reference })
    }

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }
        let response = match self.http_client.get(format!("{}/v2/settlement/list", self.base_url))
            .basic_auth(&self.store_id, Some(&self.api_key))
            .query(&[("from", req.from_date.to_string()), ("to", req.to_date.to_string())])
            .send().await
        {
            Ok(r) => r, Err(e) => { self.circuit_breaker.record_failure().await; if e.is_timeout() { return Err(ConnectorError::Timeout); } return Err(ConnectorError::NetworkError(format!("Settlement poll failed: {e}"))); }
        };
        let status = response.status();
        let resp: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;
        if !status.is_success() { self.circuit_breaker.record_failure().await; return Err(ConnectorError::Declined(resp["error"]["message"].as_str().unwrap_or("Settlement poll failed").to_string())); }
        self.circuit_breaker.record_success().await;
        let records = resp["settlements"].as_array().map(|arr| {
            arr.iter().filter_map(|item| {
                Some(RawSettlementRecord {
                    acquirer_reference: item["order_id"].as_str()?.to_string(),
                    amount: Money { amount_minor_units: (item["amount"].as_f64()? * 100.0) as i64, currency: CurrencyCode::new(item["currency"].as_str().unwrap_or("AED")).ok()? },
                    settled_at: chrono::Utc::now(),
                    fee: item["fee"].as_f64().map(|f| Money { amount_minor_units: (f * 100.0) as i64, currency: CurrencyCode::new("AED").unwrap() }),
                })
            }).collect()
        }).unwrap_or_default();
        Ok(records)
    }

    fn verify_webhook_signature(&self, headers: &axum::http::HeaderMap, body: &[u8]) -> Result<(), ConnectorError> {
        let sig = headers.get("x-telr-signature").and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing x-telr-signature header".into()))?;
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_key.as_bytes())
            .map_err(|e| ConnectorError::InvalidRequest(format!("HMAC key error: {e}")))?;
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        if sig != expected { return Err(ConnectorError::InvalidRequest("Webhook signature invalid".into())); }
        Ok(())
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let payload: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid JSON: {e}")))?;
        let event_type = payload["event"].as_str().unwrap_or("unknown").to_string();
        let acquirer_reference = payload["order"]["id"].as_str().unwrap_or("").to_string();
        let amount = payload["order"]["amount"].as_str().and_then(|s| s.parse::<f64>().ok()).map(|a| Money {
            amount_minor_units: (a * 100.0) as i64,
            currency: CurrencyCode::new(payload["order"]["currency"].as_str().unwrap_or("AED")).unwrap(),
        });
        Ok(ConnectorEvent { event_type, acquirer_reference, amount, metadata: Some(payload) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connector() -> TelrConnector {
        TelrConnector::new("telr".into(), "https://secure.telr.com".into(), "store_123".into(), "api_key_test".into())
    }

    #[test]
    fn test_connector_id() { assert_eq!(make_connector().connector_id(), "telr"); }

    #[tokio::test]
    async fn test_authorize_empty_card_token() {
        let c = make_connector();
        let req = AuthorizeRequest { card_token: "".into(), amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() }, currency: CurrencyCode::new("AED").unwrap(), merchant_reference: "ref".into(), idempotency_key: "idem".into(), metadata: None };
        assert!(matches!(c.authorize(req).await, Err(ConnectorError::InvalidRequest(_))));
    }

    #[test]
    fn test_parse_webhook() {
        let c = make_connector();
        let body = serde_json::json!({"event": "order.captured", "order": {"id": "12345", "status": "Captured", "amount": "100.00", "currency": "AED"}});
        let event = c.parse_webhook(body.to_string().as_bytes()).unwrap();
        assert_eq!(event.event_type, "order.captured");
        assert_eq!(event.acquirer_reference, "12345");
    }
}

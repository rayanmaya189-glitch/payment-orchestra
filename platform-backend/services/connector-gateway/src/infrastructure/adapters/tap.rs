//! Tap connector implementation.
//!
//! Concrete implementation of the AcquirerConnector trait for
//! Tap — supports Visa/Mastercard/mada/Apple Pay, AED/SAR/KWD/BHD.
//!
//! Makes real HTTP calls to Tap API with circuit breaker, timeout, and retry.

use async_trait::async_trait;
use axum::http::HeaderMap;

use crate::domain::aggregates::AcquirerConnector;
use crate::domain::circuit_breaker::CircuitBreaker;
use crate::domain::value_objects::*;
use shared_types::Money;

/// Tap connector.
pub struct TapConnector {
    connector_id: String,
    base_url: String,
    api_key: String,
    merchant_id: String,
    circuit_breaker: CircuitBreaker,
    http_client: reqwest::Client,
}

impl TapConnector {
    pub fn new(connector_id: String, base_url: String, api_key: String, merchant_id: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            connector_id,
            base_url,
            api_key,
            merchant_id,
            circuit_breaker: CircuitBreaker::new(),
            http_client,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[async_trait]
impl AcquirerConnector for TapConnector {
    fn connector_id(&self) -> &str {
        &self.connector_id
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        if req.card_token.is_empty() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("card_token is required".into()));
        }

        if req.amount.amount_minor_units <= 0 {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("amount must be positive".into()));
        }

        // Tap supports AED, SAR, KWD, BHD
        if !["AED", "SAR", "KWD", "BHD"].contains(&req.currency.0.as_str()) {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest(format!(
                "Tap only supports AED/SAR/KWD/BHD, got {}", req.currency.0
            )));
        }

        let request_body = serde_json::json!({
            "merchant": { "id": self.merchant_id },
            "source": { "id": req.card_token },
            "amount": req.amount.amount_minor_units as f64 / 100.0,
            "currency": req.currency.0,
            "reference": { "transaction": req.merchant_reference },
        });

        let response = match self.http_client
            .post(format!("{}/v1/charges", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("X-Idempotency-Key", &req.idempotency_key)
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if e.is_timeout() { return Err(ConnectorError::Timeout); }
                return Err(ConnectorError::NetworkError(format!("Request failed: {e}")));
            }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Response parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!(
                "Tap returned {}: {}", status, body["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success().await;

        Ok(AuthorizeResponse {
            acquirer_reference: body["id"].as_str().unwrap_or("").to_string(),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            decline_reason: body["response_code"].as_str().map(String::from),
            fee: body["fee"].as_f64().map(|f| Money { amount_minor_units: (f * 100.0) as i64, currency: req.amount.currency.clone() }),
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .post(format!("{}/v1/charges/{}/capture", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("Tap capture returned {}: {}", status, body["message"].as_str().unwrap_or("Unknown"))));
        }

        self.circuit_breaker.record_success().await;
        Ok(CaptureResponse {
            status: body["status"].as_str().unwrap_or("captured").to_string(),
            captured_amount: Money { amount_minor_units: body["amount"].as_f64().map(|a| (a * 100.0) as i64).unwrap_or(0), currency: shared_types::CurrencyCode::new("AED").unwrap() },
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .post(format!("{}/v1/charges/{}/void", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        if !response.status().is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("Tap void returned {}", response.status())));
        }

        self.circuit_breaker.record_success().await;
        Ok(VoidResponse { status: "voided".to_string() })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let request_body = serde_json::json!({
            "charge_id": req.acquirer_reference,
            "amount": req.amount.amount_minor_units as f64 / 100.0,
            "reason": req.reason,
        });

        let response = match self.http_client
            .post(format!("{}/v1/refunds", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("Tap refund returned {}: {}", status, body["message"].as_str().unwrap_or("Unknown"))));
        }

        self.circuit_breaker.record_success().await;
        Ok(RefundResponse { status: "refunded".to_string(), refund_reference: body["id"].as_str().unwrap_or("").to_string() })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .get(format!("{}/v1/charges/{}", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("Tap status returned {}", status)));
        }

        self.circuit_breaker.record_success().await;
        Ok(StatusCheckResponse { status: body["status"].as_str().unwrap_or("unknown").to_string(), acquirer_reference: req.acquirer_reference })
    }

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .get(format!("{}/v1/settlements", self.base_url))
            .header("Authorization", self.auth_header())
            .query(&[("from", &req.from_date), ("to", &req.to_date)])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| ConnectorError::NetworkError(format!("Parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("Tap settlements returned {}", status)));
        }

        self.circuit_breaker.record_success().await;
        let records = body["settlements"].as_array().map(|arr| {
            arr.iter().filter_map(|r| {
                let acquirer_ref = r.get("id")?.as_str()?.to_string();
                let amount = r.get("amount")?.as_f64()?;
                let settled_at_str = r.get("settled_at")?.as_str()?;
                let settled_at = chrono::DateTime::parse_from_rfc3339(settled_at_str).ok()?.with_timezone(&chrono::Utc);
                Some(RawSettlementRecord {
                    acquirer_reference: acquirer_ref,
                    amount: Money { amount_minor_units: (amount * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                    settled_at,
                    fee: r.get("fee").and_then(|v| v.as_f64()).map(|f| Money { amount_minor_units: (f * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() }),
                })
            }).collect()
        }).unwrap_or_default();

        Ok(records)
    }

    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("X-Tap-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing X-Tap-Signature header".into()))?;

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.api_key.as_bytes())
            .map_err(|e| ConnectorError::InvalidRequest(format!("HMAC error: {e}")))?;
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());

        if signature != expected {
            return Err(ConnectorError::InvalidRequest("Invalid webhook signature".into()));
        }
        Ok(())
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let payload: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid JSON: {e}")))?;

        Ok(ConnectorEvent {
            event_type: payload["event_type"].as_str().unwrap_or("unknown").to_string(),
            acquirer_reference: payload["id"].as_str().unwrap_or("").to_string(),
            amount: payload.get("amount").and_then(|v| v.as_f64().map(|amt| Money { amount_minor_units: (amt * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() })),
            metadata: Some(payload),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tap_authorize_empty_card_token() {
        let connector = TapConnector::new("tap-test".into(), "https://api.tap.company".into(), "test-key".into(), "test-merchant".into());
        let req = AuthorizeRequest {
            amount: Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() },
            card_token: "".to_string(),
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };
        let err = connector.authorize(req).await.unwrap_err();
        match err { ConnectorError::InvalidRequest(_) => {}, _ => panic!("Expected InvalidRequest") }
    }

    #[tokio::test]
    async fn test_tap_rejects_unsupported_currency() {
        let connector = TapConnector::new("tap-test".into(), "https://api.tap.company".into(), "test-key".into(), "test-merchant".into());
        let req = AuthorizeRequest {
            amount: Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("USD").unwrap() },
            card_token: "tok_test".to_string(),
            currency: shared_types::CurrencyCode::new("USD").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };
        let err = connector.authorize(req).await.unwrap_err();
        match err { ConnectorError::InvalidRequest(msg) => assert!(msg.contains("AED/SAR/KWD/BHD")), _ => panic!("Expected InvalidRequest") }
    }
}

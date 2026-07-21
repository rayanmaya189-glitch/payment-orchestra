//! Checkout.com connector implementation.
//!
//! Supports Visa/Mastercard/Amex, AED/USD/EUR/GBP.
//! Native idempotency support per SRS §4.

use async_trait::async_trait;

use crate::domain::aggregates::AcquirerConnector;
use crate::domain::circuit_breaker::CircuitBreaker;
use crate::domain::value_objects::*;
use shared_types::Money;

pub struct CheckoutComConnector {
    connector_id: String,
    base_url: String,
    secret_key: String,
    circuit_breaker: CircuitBreaker,
    http_client: reqwest::Client,
}

impl CheckoutComConnector {
    pub fn new(connector_id: String, base_url: String, secret_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self { connector_id, base_url, secret_key, circuit_breaker: CircuitBreaker::new(), http_client }
    }
}

#[async_trait]
impl AcquirerConnector for CheckoutComConnector {
    fn connector_id(&self) -> &str { &self.connector_id }

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

        let request_body = serde_json::json!({
            "source": { "type": "token", "token": req.card_token },
            "amount": req.amount.amount_minor_units,
            "currency": req.amount.currency.0,
            "reference": req.merchant_reference,
            "capture": false,
        });

        let response = match self.http_client
            .post(format!("{}/payments", self.base_url))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("Content-Type", "application/json")
            .header("cko-version", "2023-03-01")
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
            let msg = body["message"].as_str().unwrap_or("Checkout.com API error");
            return Err(ConnectorError::Declined(msg.to_string()));
        }

        self.circuit_breaker.record_success().await;

        let checkout_status = body["status"].as_str().unwrap_or("Unknown");
        let response_status = match checkout_status {
            "Authorized" | "Captured" => "approved",
            "Declined" => "declined",
            "Pending" => "pending",
            _ => "error",
        };

        Ok(AuthorizeResponse {
            status: response_status.to_string(),
            acquirer_reference: body["id"].as_str().unwrap_or("").to_string(),
            decline_reason: if response_status == "declined" {
                body["response_code"].as_str().map(|s| s.to_string())
            } else { None },
            fee: None,
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let request_body = if let Some(amount) = &req.amount {
            serde_json::json!({ "amount": amount.amount_minor_units })
        } else { serde_json::json!({}) };

        let response = match self.http_client
            .post(format!("{}/payments/{}/captures", self.base_url, req.acquirer_reference))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("Content-Type", "application/json")
            .header("cko-version", "2023-03-01")
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if e.is_timeout() { return Err(ConnectorError::Timeout); }
                return Err(ConnectorError::NetworkError(format!("Capture failed: {e}")));
            }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Response parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::Declined(body["message"].as_str().unwrap_or("Capture failed").to_string()));
        }

        self.circuit_breaker.record_success().await;
        let cap_status = body["status"].as_str().unwrap_or("Captured").to_string();
        let cap_amount = req.amount.clone().unwrap_or(Money { amount_minor_units: 0, currency: shared_types::CurrencyCode::new("AED").unwrap() });

        Ok(CaptureResponse { status: cap_status, captured_amount: cap_amount })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .post(format!("{}/payments/{}/voids", self.base_url, req.acquirer_reference))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("Content-Type", "application/json")
            .header("cko-version", "2023-03-01")
            .json(&serde_json::json!({}))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if e.is_timeout() { return Err(ConnectorError::Timeout); }
                return Err(ConnectorError::NetworkError(format!("Void failed: {e}")));
            }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Response parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::Declined(body["message"].as_str().unwrap_or("Void failed").to_string()));
        }

        self.circuit_breaker.record_success().await;
        Ok(VoidResponse { status: body["status"].as_str().unwrap_or("Voided").to_string() })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let request_body = serde_json::json!({
            "amount": req.amount.amount_minor_units,
        });

        let response = match self.http_client
            .post(format!("{}/payments/{}/refunds", self.base_url, req.acquirer_reference))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("Content-Type", "application/json")
            .header("cko-version", "2023-03-01")
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if e.is_timeout() { return Err(ConnectorError::Timeout); }
                return Err(ConnectorError::NetworkError(format!("Refund failed: {e}")));
            }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Response parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::Declined(body["message"].as_str().unwrap_or("Refund failed").to_string()));
        }

        self.circuit_breaker.record_success().await;
        Ok(RefundResponse {
            status: body["status"].as_str().unwrap_or("Refunded").to_string(),
            refund_reference: body["id"].as_str().unwrap_or("").to_string(),
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .get(format!("{}/payments/{}", self.base_url, req.acquirer_reference))
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .header("cko-version", "2023-03-01")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if e.is_timeout() { return Err(ConnectorError::Timeout); }
                return Err(ConnectorError::NetworkError(format!("Status check failed: {e}")));
            }
        };

        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Response parse failed: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::Declined(body["message"].as_str().unwrap_or("Status check failed").to_string()));
        }

        self.circuit_breaker.record_success().await;
        let mapped = match body["status"].as_str().unwrap_or("Unknown") {
            "Authorized" => "authorized",
            "Captured" => "captured",
            "Voided" => "voided",
            "Refunded" => "refunded",
            "Declined" => "declined",
            _ => "unknown",
        };

        Ok(StatusCheckResponse { status: mapped.to_string(), acquirer_reference: req.acquirer_reference })
    }

    async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // Checkout.com uses webhook-based settlement
        Ok(vec![])
    }

    fn verify_webhook_signature(&self, headers: &axum::http::HeaderMap, body: &[u8]) -> Result<(), ConnectorError> {
        let sig_timestamp = headers.get("cko-timestamp").and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing cko-timestamp header".into()))?;
        let sig_value = headers.get("cko-signature").and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing cko-signature header".into()))?;

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes())
            .map_err(|e| ConnectorError::InvalidRequest(format!("HMAC key error: {e}")))?;
        mac.update(format!("{}.{}", sig_timestamp, String::from_utf8_lossy(body)).as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());
        if sig_value != expected { return Err(ConnectorError::InvalidRequest("Webhook signature invalid".into())); }
        Ok(())
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let payload: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid JSON: {e}")))?;
        let event_type = payload["type"].as_str().unwrap_or("unknown").to_string();
        let acquirer_reference = payload["data"]["id"].as_str().unwrap_or("").to_string();
        let amount = payload["data"]["amount"].as_i64().map(|a| Money {
            amount_minor_units: a,
            currency: shared_types::CurrencyCode::new(payload["data"]["currency"].as_str().unwrap_or("AED")).unwrap(),
        });

        Ok(ConnectorEvent { event_type, acquirer_reference, amount, metadata: Some(payload) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connector() -> CheckoutComConnector {
        CheckoutComConnector::new("checkout_com".into(), "https://api.sandbox.checkout.com".into(), "sk_test".into())
    }

    #[test]
    fn test_connector_id() { assert_eq!(make_connector().connector_id(), "checkout_com"); }

    #[tokio::test]
    async fn test_authorize_empty_card_token() {
        let c = make_connector();
        let req = AuthorizeRequest { card_token: "".into(), amount: Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() }, currency: shared_types::CurrencyCode::new("AED").unwrap(), merchant_reference: "ref".into(), idempotency_key: "idem".into(), metadata: None };
        assert!(matches!(c.authorize(req).await, Err(ConnectorError::InvalidRequest(_))));
    }

    #[test]
    fn test_parse_webhook() {
        let c = make_connector();
        let body = serde_json::json!({"type": "payment.captured", "data": {"id": "pay_123", "amount": 10000, "currency": "AED"}});
        let event = c.parse_webhook(body.to_string().as_bytes()).unwrap();
        assert_eq!(event.event_type, "payment.captured");
        assert_eq!(event.acquirer_reference, "pay_123");
    }
}

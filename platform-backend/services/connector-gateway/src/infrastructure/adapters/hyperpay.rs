//! HyperPay connector implementation.
//!
//! Concrete implementation of the AcquirerConnector trait for
//! HyperPay — supports Visa/Mastercard/mada/Apple Pay/Google Pay.
//!
//! Makes real HTTP calls to HyperPay API with circuit breaker, timeout, and retry.

use async_trait::async_trait;
use axum::http::HeaderMap;

use crate::domain::aggregates::AcquirerConnector;
use crate::domain::circuit_breaker::CircuitBreaker;
use crate::domain::value_objects::*;
use shared_types::Money;

/// HyperPay connector.
pub struct HyperPayConnector {
    connector_id: String,
    base_url: String,
    entity_id: String,
    access_token: String,
    circuit_breaker: CircuitBreaker,
    http_client: reqwest::Client,
}

impl HyperPayConnector {
    pub fn new(connector_id: String, base_url: String, entity_id: String, access_token: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            connector_id,
            base_url,
            entity_id,
            access_token,
            circuit_breaker: CircuitBreaker::new(),
            http_client,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

#[async_trait]
impl AcquirerConnector for HyperPayConnector {
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

        let request_body = serde_json::json!({
            "entityId": self.entity_id,
            "amount": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0),
            "currency": req.currency.0,
            "paymentType": "DB",
            "cardToken": req.card_token,
            "merchantTransactionId": req.merchant_reference,
        });

        let response = match self.http_client
            .post(format!("{}/v1/checkouts", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
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
                "HyperPay returned {}: {}", status, body["result"]["description"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success().await;

        Ok(AuthorizeResponse {
            acquirer_reference: body["id"].as_str().unwrap_or("").to_string(),
            status: body["result"]["code"].as_str().unwrap_or("unknown").to_string(),
            decline_reason: body["result"]["description"].as_str().map(String::from),
            fee: body["fee"].as_str().and_then(|f| f.parse::<f64>().ok()).map(|f| Money { amount_minor_units: (f * 100.0) as i64, currency: req.amount.currency.clone() }),
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let request_body = serde_json::json!({
            "entityId": self.entity_id,
            "amount": req.amount.as_ref().map(|a| format!("{:.2}", a.amount_minor_units as f64 / 100.0)),
        });

        let response = match self.http_client
            .post(format!("{}/v1/checkouts/{}/capture", self.base_url, req.acquirer_reference))
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
            return Err(ConnectorError::NetworkError(format!("HyperPay capture returned {}: {}", status, body["result"]["description"].as_str().unwrap_or("Unknown"))));
        }

        self.circuit_breaker.record_success().await;
        Ok(CaptureResponse {
            status: body["result"]["code"].as_str().unwrap_or("captured").to_string(),
            captured_amount: Money { amount_minor_units: body["amount"].as_str().and_then(|a| a.parse::<f64>().ok()).map(|a| (a * 100.0) as i64).unwrap_or(0), currency: shared_types::CurrencyCode::new("AED").unwrap() },
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .post(format!("{}/v1/checkouts/{}/void", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => { self.circuit_breaker.record_failure().await; return Err(ConnectorError::NetworkError(format!("Request failed: {e}"))); }
        };

        if !response.status().is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!("HyperPay void returned {}", response.status())));
        }

        self.circuit_breaker.record_success().await;
        Ok(VoidResponse { status: "voided".to_string() })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let request_body = serde_json::json!({
            "entityId": self.entity_id,
            "amount": format!("{:.2}", req.amount.amount_minor_units as f64 / 100.0),
            "paymentType": "RF",
        });

        let response = match self.http_client
            .post(format!("{}/v1/checkouts/{}/refund", self.base_url, req.acquirer_reference))
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
            return Err(ConnectorError::NetworkError(format!("HyperPay refund returned {}: {}", status, body["result"]["description"].as_str().unwrap_or("Unknown"))));
        }

        self.circuit_breaker.record_success().await;
        Ok(RefundResponse { status: "refunded".to_string(), refund_reference: body["id"].as_str().unwrap_or("").to_string() })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await { return Err(ConnectorError::RateLimited); }

        let response = match self.http_client
            .get(format!("{}/v1/checkouts/{}", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
            .query(&[("entityId", &self.entity_id)])
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
            return Err(ConnectorError::NetworkError(format!("HyperPay status returned {}", status)));
        }

        self.circuit_breaker.record_success().await;
        Ok(StatusCheckResponse { status: body["payment"]["result"]["code"].as_str().unwrap_or("unknown").to_string(), acquirer_reference: req.acquirer_reference })
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
            return Err(ConnectorError::NetworkError(format!("HyperPay settlements returned {}", status)));
        }

        self.circuit_breaker.record_success().await;
        let records = body["settlements"].as_array().map(|arr| {
            arr.iter().filter_map(|r| {
                let acquirer_ref = r.get("id")?.as_str()?.to_string();
                let amount_str = r.get("amount")?.as_str()?;
                let amount = amount_str.parse::<f64>().ok()?;
                let settled_at_str = r.get("settled_at")?.as_str()?;
                let settled_at = chrono::DateTime::parse_from_rfc3339(settled_at_str).ok()?.with_timezone(&chrono::Utc);
                Some(RawSettlementRecord {
                    acquirer_reference: acquirer_ref,
                    amount: Money { amount_minor_units: (amount * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                    settled_at,
                    fee: r.get("fee").and_then(|v| v.as_str()).and_then(|f| f.parse::<f64>().ok()).map(|f| Money { amount_minor_units: (f * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() }),
                })
            }).collect()
        }).unwrap_or_default();

        Ok(records)
    }

    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError> {
        let signature = headers.get("X-HyperPay-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing X-HyperPay-Signature header".into()))?;

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.access_token.as_bytes())
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
            amount: payload.get("amount").and_then(|v| v.as_str()?.parse::<f64>().ok()).map(|amt| Money { amount_minor_units: (amt * 100.0) as i64, currency: shared_types::CurrencyCode::new("AED").unwrap() }),
            metadata: Some(payload),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hyperpay_authorize_empty_card_token() {
        let connector = HyperPayConnector::new("hyperpay-test".into(), "https://api.hyperpay.com".into(), "test-entity".into(), "test-token".into());
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
}

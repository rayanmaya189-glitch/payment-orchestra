//! Network International connector implementation.
//!
//! Concrete implementation of the AcquirerConnector trait for
//! Network International (NI) — supports Visa/Mastercard, AED only.
//!
//! Makes real HTTP calls to NI API with circuit breaker, timeout, and retry.

use async_trait::async_trait;
use axum::http::HeaderMap;
use uuid::Uuid;

use crate::domain::aggregates::AcquirerConnector;
use crate::domain::circuit_breaker::CircuitBreaker;
use crate::domain::value_objects::*;
use shared_types::Money;

/// Network International connector.
pub struct NetworkInternationalConnector {
    connector_id: String,
    base_url: String,
    api_key: String,
    circuit_breaker: CircuitBreaker,
    http_client: reqwest::Client,
}

impl NetworkInternationalConnector {
    pub fn new(connector_id: String, base_url: String, api_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            connector_id,
            base_url,
            api_key,
            circuit_breaker: CircuitBreaker::new(),
            http_client,
        }
    }

    /// Build authorization header for NI API.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[async_trait]
impl AcquirerConnector for NetworkInternationalConnector {
    fn connector_id(&self) -> &str {
        &self.connector_id
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        // Check circuit breaker
        if !self.circuit_breaker.allow_request().await {
            tracing::warn!(connector = %self.connector_id, "Circuit breaker open — rejecting authorize");
            return Err(ConnectorError::RateLimited);
        }

        // Validate card token format
        if req.card_token.is_empty() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("card_token is required".into()));
        }

        // Validate amount
        if req.amount.amount_minor_units <= 0 {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("amount must be positive".into()));
        }

        // Validate currency (NI only supports AED)
        if req.currency.0 != "AED" {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest(format!(
                "NI only supports AED, got {}", req.currency.0
            )));
        }

        // Build NI API request
        let request_body = serde_json::json!({
            "card_token": req.card_token,
            "amount": req.amount.amount_minor_units,
            "currency": req.currency.0,
            "merchant_reference": req.merchant_reference,
            "idempotency_key": req.idempotency_key,
        });

        tracing::info!(
            connector = %self.connector_id,
            amount = req.amount.amount_minor_units,
            currency = %req.currency.0,
            "Sending authorize request to NI API"
        );

        // Make HTTP call to NI API
        let response = match self.http_client
            .post(format!("{}/api/v1/transactions/authorize", self.base_url))
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
                if e.is_timeout() {
                    return Err(ConnectorError::Timeout);
                } else if e.is_connect() {
                    return Err(ConnectorError::NetworkError(format!("Connection failed: {e}")));
                } else {
                    return Err(ConnectorError::NetworkError(format!("Request failed: {e}")));
                }
            }
        };

        // Parse response
        let status = response.status();
        let body: serde_json::Value = response.json().await
            .map_err(|e| ConnectorError::NetworkError(format!("Failed to parse response: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            return Err(ConnectorError::NetworkError(format!(
                "NI API returned {}: {}", status, error_msg
            )));
        }

        self.circuit_breaker.record_success();

        let acquirer_ref = body["transaction_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let ni_status = body["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %acquirer_ref,
            status = %ni_status,
            "NI authorize response received"
        );

        Ok(AuthorizeResponse {
            acquirer_reference: acquirer_ref,
            status: ni_status,
            decline_reason: body["decline_reason"].as_str().map(String::from),
            fee: body["fee"].as_i64().map(|f| Money {
                amount_minor_units: f,
                currency: req.amount.currency.clone(),
            }),
        })
    }

    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        if req.acquirer_reference.is_empty() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("acquirer_reference is required".into()));
        }

        let request_body = serde_json::json!({
            "transaction_id": req.acquirer_reference,
            "amount": req.amount.as_ref().map(|a| a.amount_minor_units),
        });

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Sending capture request to NI API"
        );

        let response = match self.http_client
            .post(format!("{}/api/v1/transactions/capture", self.base_url))
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
            .map_err(|e| ConnectorError::NetworkError(format!("Failed to parse response: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!(
                "NI capture returned {}: {}", status,
                body["error"]["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success();

        let captured_amount = body["captured_amount"].as_i64().unwrap_or(0);

        Ok(CaptureResponse {
            status: body["status"].as_str().unwrap_or("captured").to_string(),
            captured_amount: Money {
                amount_minor_units: captured_amount,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            },
        })
    }

    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        if req.acquirer_reference.is_empty() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("acquirer_reference is required".into()));
        }

        let request_body = serde_json::json!({
            "transaction_id": req.acquirer_reference,
        });

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Sending void request to NI API"
        );

        let response = match self.http_client
            .post(format!("{}/api/v1/transactions/void", self.base_url))
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
        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            return Err(ConnectorError::NetworkError(format!(
                "NI void returned {}: {}", status,
                body["error"]["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success();

        Ok(VoidResponse {
            status: "voided".to_string(),
        })
    }

    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        if req.acquirer_reference.is_empty() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("acquirer_reference is required".into()));
        }

        if req.amount.amount_minor_units <= 0 {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::InvalidRequest("refund amount must be positive".into()));
        }

        let request_body = serde_json::json!({
            "transaction_id": req.acquirer_reference,
            "amount": req.amount.amount_minor_units,
            "reason": req.reason,
        });

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            amount = req.amount.amount_minor_units,
            "Sending refund request to NI API"
        );

        let response = match self.http_client
            .post(format!("{}/api/v1/transactions/refund", self.base_url))
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
            .map_err(|e| ConnectorError::NetworkError(format!("Failed to parse response: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!(
                "NI refund returned {}: {}", status,
                body["error"]["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success();

        Ok(RefundResponse {
            status: "refunded".to_string(),
            refund_reference: body["refund_id"].as_str().unwrap_or("").to_string(),
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Sending status check to NI API"
        );

        let response = match self.http_client
            .get(format!("{}/api/v1/transactions/{}", self.base_url, req.acquirer_reference))
            .header("Authorization", self.auth_header())
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
            .map_err(|e| ConnectorError::NetworkError(format!("Failed to parse response: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!(
                "NI status check returned {}: {}", status,
                body["error"]["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success();

        Ok(StatusCheckResponse {
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            acquirer_reference: req.acquirer_reference,
        })
    }

    async fn poll_settlement(
        &self,
        req: PollSettlementRequest,
    ) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        tracing::info!(
            connector = %self.connector_id,
            from = %req.from_date,
            to = %req.to_date,
            "Polling settlement records from NI API"
        );

        let response = match self.http_client
            .get(format!("{}/api/v1/settlements", self.base_url))
            .header("Authorization", self.auth_header())
            .query(&[
                ("from_date", req.from_date.to_string()),
                ("to_date", req.to_date.to_string()),
            ])
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
            .map_err(|e| ConnectorError::NetworkError(format!("Failed to parse response: {e}")))?;

        if !status.is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(ConnectorError::NetworkError(format!(
                "NI settlement poll returned {}: {}", status,
                body["error"]["message"].as_str().unwrap_or("Unknown error")
            )));
        }

        self.circuit_breaker.record_success();

        let records = body["records"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(RawSettlementRecord {
                            acquirer_reference: r["transaction_id"].as_str()?.to_string(),
                            amount: Money {
                                amount_minor_units: r["amount"].as_i64()?,
                                currency: shared_types::CurrencyCode::new("AED").unwrap(),
                            },
                            settled_at: chrono::DateTime::parse_from_rfc3339(
                                r["settled_at"].as_str()?
                            ).ok()?.with_timezone(&chrono::Utc),
                            fee: r["fee"].as_i64().map(|f| Money {
                                amount_minor_units: f,
                                currency: shared_types::CurrencyCode::new("AED").unwrap(),
                            }),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(records)
    }

    fn verify_webhook_signature(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<(), ConnectorError> {
        // NI uses X-NI-Signature header with HMAC-SHA256
        let signature = headers
            .get("X-NI-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ConnectorError::InvalidRequest("Missing X-NI-Signature header".into()))?;

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.api_key.as_bytes())
            .map_err(|e| ConnectorError::InvalidRequest(format!("HMAC key error: {e}")))?;
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

        let event_type = payload["event_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let acquirer_reference = payload["transaction_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ConnectorEvent {
            event_type,
            acquirer_reference,
            amount: payload.get("amount").and_then(|v| {
                v.as_i64().map(|amt| Money {
                    amount_minor_units: amt,
                    currency: shared_types::CurrencyCode::new("AED").unwrap(),
                })
            }),
            metadata: Some(payload),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ni_authorize_empty_card_token() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "https://api.test.ni.com".to_string(),
            "test-key".to_string(),
        );

        let req = AuthorizeRequest {
            amount: Money {
                amount_minor_units: 10000,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            },
            card_token: "".to_string(),
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };

        let err = connector.authorize(req).await.unwrap_err();
        match err {
            ConnectorError::InvalidRequest(msg) => assert!(msg.contains("card_token")),
            _ => panic!("Expected InvalidRequest"),
        }
    }

    #[tokio::test]
    async fn test_ni_authorize_zero_amount() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "https://api.test.ni.com".to_string(),
            "test-key".to_string(),
        );

        let req = AuthorizeRequest {
            amount: Money {
                amount_minor_units: 0,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            },
            card_token: "tok_test_123".to_string(),
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };

        let err = connector.authorize(req).await.unwrap_err();
        match err {
            ConnectorError::InvalidRequest(msg) => assert!(msg.contains("positive")),
            _ => panic!("Expected InvalidRequest"),
        }
    }

    #[tokio::test]
    async fn test_ni_authorize_rejects_non_aed() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "https://api.test.ni.com".to_string(),
            "test-key".to_string(),
        );

        let req = AuthorizeRequest {
            amount: Money {
                amount_minor_units: 10000,
                currency: shared_types::CurrencyCode::new("USD").unwrap(),
            },
            card_token: "tok_test_123".to_string(),
            currency: shared_types::CurrencyCode::new("USD").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };

        let err = connector.authorize(req).await.unwrap_err();
        match err {
            ConnectorError::InvalidRequest(msg) => assert!(msg.contains("AED")),
            _ => panic!("Expected InvalidRequest"),
        }
    }

    #[tokio::test]
    async fn test_ni_authorize_network_error() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "http://localhost:99999".to_string(), // Non-existent server
            "test-key".to_string(),
        );

        let req = AuthorizeRequest {
            amount: Money {
                amount_minor_units: 10000,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            },
            card_token: "tok_test_123".to_string(),
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
            merchant_reference: "ord_123".to_string(),
            idempotency_key: "idem_test".to_string(),
            metadata: None,
        };

        let err = connector.authorize(req).await.unwrap_err();
        match err {
            ConnectorError::NetworkError(_) => {} // Expected
            ConnectorError::Timeout => {} // Also acceptable
            _ => panic!("Expected NetworkError or Timeout, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_ni_webhook_parse() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "https://api.test.ni.com".to_string(),
            "test-key".to_string(),
        );

        let body = serde_json::json!({
            "event_type": "payment.captured",
            "transaction_id": "NI_abc123",
            "amount": 5000
        });

        let event = connector.parse_webhook(body.to_string().as_bytes()).unwrap();
        assert_eq!(event.event_type, "payment.captured");
        assert_eq!(event.acquirer_reference, "NI_abc123");
    }

    #[tokio::test]
    async fn test_ni_circuit_breaker() {
        let connector = NetworkInternationalConnector::new(
            "ni-test".to_string(),
            "http://localhost:99999".to_string(),
            "test-key".to_string(),
        );

        // Make several failed requests to trip the circuit breaker
        // Need at least 10 requests for the threshold check
        for _ in 0..10 {
            let req = AuthorizeRequest {
                amount: Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                card_token: "tok_test".to_string(),
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
                merchant_reference: "test".to_string(),
                idempotency_key: "idem_test".to_string(),
                metadata: None,
            };
            let _ = connector.authorize(req).await;
        }

        // Circuit breaker should be open now
        assert!(!connector.circuit_breaker.allow_request().await);
    }
}
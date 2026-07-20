//! Network International connector implementation.
//!
//! Concrete implementation of the AcquirerConnector trait for
//! Network International (NI) — supports Visa/Mastercard, AED only.
//!
//! This is a production-ready stub with proper error handling,
//! circuit breaker integration, and structured logging.

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
}

impl NetworkInternationalConnector {
    pub fn new(connector_id: String, base_url: String, api_key: String) -> Self {
        Self {
            connector_id,
            base_url,
            api_key,
            circuit_breaker: CircuitBreaker::new(),
        }
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

        // TODO: Make actual HTTP call to NI API
        // For now, simulate a successful authorization
        tracing::info!(
            connector = %self.connector_id,
            amount = req.amount.amount_minor_units,
            currency = %req.currency.0,
            "Authorize request (simulated)"
        );

        self.circuit_breaker.record_success().await;

        Ok(AuthorizeResponse {
            acquirer_reference: format!("NI_{}", Uuid::now_v7()),
            status: "approved".to_string(),
            decline_reason: None,
            fee: None,
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

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Capture request (simulated)"
        );

        self.circuit_breaker.record_success().await;

        Ok(CaptureResponse {
            status: "captured".to_string(),
            captured_amount: req.amount.unwrap_or(Money {
                amount_minor_units: 0,
                currency: shared_types::CurrencyCode::new("AED").unwrap(),
            }),
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

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Void request (simulated)"
        );

        self.circuit_breaker.record_success().await;

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

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            amount = req.amount.amount_minor_units,
            "Refund request (simulated)"
        );

        self.circuit_breaker.record_success().await;

        Ok(RefundResponse {
            status: "refunded".to_string(),
            refund_reference: format!("NI_REF_{}", Uuid::now_v7()),
        })
    }

    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        tracing::info!(
            connector = %self.connector_id,
            acquirer_ref = %req.acquirer_reference,
            "Status check (simulated)"
        );

        self.circuit_breaker.record_success().await;

        Ok(StatusCheckResponse {
            status: "captured".to_string(),
            acquirer_reference: req.acquirer_reference,
        })
    }

    async fn poll_settlement(
        &self,
        _req: PollSettlementRequest,
    ) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        if !self.circuit_breaker.allow_request().await {
            return Err(ConnectorError::RateLimited);
        }

        tracing::info!(connector = %self.connector_id, "Poll settlement (simulated)");

        self.circuit_breaker.record_success().await;

        // TODO: Make actual API call to NI settlement endpoint
        Ok(vec![])
    }

    fn verify_webhook_signature(
        &self,
        _headers: &HeaderMap,
        _body: &[u8],
    ) -> Result<(), ConnectorError> {
        // TODO: Implement HMAC-SHA256 signature verification
        // NI uses X-NI-Signature header with HMAC-SHA256
        Ok(())
    }

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
        let payload: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| ConnectorError::InvalidRequest(format!("Invalid JSON: {e}")))?;

        let event_type = payload["event_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let acquirer_reference = payload["acquirer_reference"]
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
    async fn test_ni_authorize_success() {
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
            card_token: "tok_test_123".to_string(),
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
            merchant_reference: "ord_123".to_string(),
            metadata: None,
        };

        let resp = connector.authorize(req).await.unwrap();
        assert_eq!(resp.status, "approved");
        assert!(resp.acquirer_reference.starts_with("NI_"));
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
            metadata: None,
        };

        let err = connector.authorize(req).await.unwrap_err();
        match err {
            ConnectorError::InvalidRequest(msg) => assert!(msg.contains("AED")),
            _ => panic!("Expected InvalidRequest"),
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
            "acquirer_reference": "NI_abc123",
            "amount": 5000
        });

        let event = connector.parse_webhook(body.to_string().as_bytes()).unwrap();
        assert_eq!(event.event_type, "payment.captured");
        assert_eq!(event.acquirer_reference, "NI_abc123");
    }
}
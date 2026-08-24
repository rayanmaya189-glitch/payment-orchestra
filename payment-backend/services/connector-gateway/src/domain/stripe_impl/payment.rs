//! Stripe payment operations: authorize, capture, void, refund, status_check.

use std::time::Instant;

use serde_json::Value;

use super::super::error::ConnectorError;
use super::super::stripe_connector::StripeConnector;
use super::super::types::*;

const STRIPE_API_VERSION: &str = "2025-02-24.acacia";

impl StripeConnector {
    pub(super) async fn authorize_impl(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let start = Instant::now();

        {
            let mut cb = self.circuit_breaker.lock().map_err(|e| {
                ConnectorError::NetworkError(format!("Circuit breaker lock: {}", e))
            })?;
            if !cb.is_call_allowed() {
                return Err(ConnectorError::CircuitBreakerOpen("stripe".into()));
            }
        }

        let params = self.build_authorize_params(&req);

        let resp = self
            .client
            .post(format!("{}/v1/payment_intents", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Stripe-Version", STRIPE_API_VERSION)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ConnectorError::Timeout(start.elapsed().as_millis() as u64)
                } else if e.is_connect() {
                    ConnectorError::NetworkError(format!("Stripe connection: {}", e))
                } else {
                    ConnectorError::NetworkError(format!("Stripe request: {}", e))
                }
            })?;

        let latency_ms = start.elapsed().as_millis() as u32;
        let status_code = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        if status_code.is_success() {
            let result = self.normalize_authorize_response(&body, latency_ms)?;
            {
                let mut cb = self.circuit_breaker.lock().map_err(|e| {
                    ConnectorError::NetworkError(format!("Circuit breaker lock: {}", e))
                })?;
                if result.status == AuthorizeStatus::Approved {
                    cb.record_success();
                } else {
                    cb.record_failure();
                }
            }
            Ok(result)
        } else {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown Stripe error");
            let decline_code = body["error"]["decline_code"]
                .as_str()
                .unwrap_or("generic_decline");
            let normalized = self.decline_table.normalize(decline_code);

            {
                let mut cb = self.circuit_breaker.lock().map_err(|e| {
                    ConnectorError::NetworkError(format!("Circuit breaker lock: {}", e))
                })?;
                cb.record_failure();
            }

            match status_code.as_u16() {
                401 => Err(ConnectorError::AuthenticationFailed(error_msg.into())),
                429 => Err(ConnectorError::RateLimited),
                _ => Err(ConnectorError::AcquirerDeclined(format!(
                    "{}: {}",
                    normalized, error_msg
                ))),
            }
        }
    }

    pub(super) async fn capture_impl(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
        let start = Instant::now();
        let pi_id = req.acquirer_reference.strip_prefix("pi_").unwrap_or(&req.acquirer_reference);

        let mut params: Vec<(&str, String)> = vec![
            ("amount_to_capture", req.amount.amount_minor_units.to_string()),
        ];
        if !req.idempotency_key.is_empty() {
            params.push(("idempotency_key", req.idempotency_key));
        }

        let resp = self
            .client
            .post(format!("{}/v1/payment_intents/{}/capture", self.base_url, pi_id))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe capture: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let status = body["status"].as_str().unwrap_or("unknown");
        let amount = body["amount"]
            .as_i64()
            .unwrap_or(req.amount.amount_minor_units);

        Ok(CaptureResponse {
            success: status == "succeeded",
            acquirer_reference: body["id"].as_str().map(|s| format!("pi_{}", s)),
            amount_captured: Money {
                amount_minor_units: amount,
                currency: req.amount.currency,
            },
            latency_ms,
        })
    }

    pub(super) async fn void_impl(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
        let start = Instant::now();
        let pi_id = req.acquirer_reference.strip_prefix("pi_").unwrap_or(&req.acquirer_reference);

        let resp = self
            .client
            .post(format!("{}/v1/payment_intents/{}/cancel", self.base_url, pi_id))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe void: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let status = body["status"].as_str().unwrap_or("unknown");

        Ok(VoidResponse {
            success: status == "canceled",
            acquirer_reference: body["id"].as_str().map(|s| format!("pi_{}", s)),
            latency_ms,
        })
    }

    pub(super) async fn refund_impl(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
        let start = Instant::now();
        let pi_id = req.acquirer_reference.strip_prefix("pi_").unwrap_or(&req.acquirer_reference);

        let mut params: Vec<(&str, String)> = vec![
            ("payment_intent", pi_id.to_string()),
            ("amount", req.amount.amount_minor_units.to_string()),
        ];
        if !req.idempotency_key.is_empty() {
            params.push(("idempotency_key", req.idempotency_key));
        }

        let resp = self
            .client
            .post(format!("{}/v1/refunds", self.base_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe refund: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let status = body["status"].as_str().unwrap_or("unknown");

        Ok(RefundResponse {
            success: status == "succeeded",
            acquirer_reference: body["id"].as_str().map(|s| format!("re_{}", s)),
            refund_id: body["id"].as_str().map(String::from),
            latency_ms,
        })
    }

    pub(super) async fn status_check_impl(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
        let start = Instant::now();
        let pi_id = req.acquirer_reference.strip_prefix("pi_").unwrap_or(&req.acquirer_reference);

        let resp = self
            .client
            .get(format!("{}/v1/payment_intents/{}", self.base_url, pi_id))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ConnectorError::NetworkError(format!("Stripe status: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u32;
        let body: Value = resp.json().await.map_err(|e| {
            ConnectorError::NetworkError(format!("Stripe parse: {}", e))
        })?;

        let status = body["status"].as_str().unwrap_or("unknown");
        let amount = body["amount"].as_i64();

        let auth_status = match status {
            "succeeded" => AuthorizeStatus::Approved,
            "requires_capture" => AuthorizeStatus::Approved,
            "processing" => AuthorizeStatus::Approved,
            "requires_action" => AuthorizeStatus::Requires3DS,
            "requires_payment_method" => AuthorizeStatus::Declined,
            "canceled" => AuthorizeStatus::Declined,
            _ => AuthorizeStatus::Declined,
        };

        Ok(StatusCheckResponse {
            status: auth_status,
            acquirer_reference: body["id"].as_str().map(|s| format!("pi_{}", s)),
            amount: amount.map(|a| Money {
                amount_minor_units: a,
                currency: body["currency"].as_str().unwrap_or("usd").to_uppercase(),
            }),
            latency_ms,
        })
    }
}

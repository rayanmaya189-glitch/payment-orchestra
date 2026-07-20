//! Connector client — real HTTP calls to connector-gateway service.
//!
//! This replaces the TODO stub in PaymentServiceImpl::authorize with actual
//! HTTP calls to the connector-gateway microservice, which then translates
//! to acquirer-specific API calls.
//!
//! SRS Part 5 §5: orchestration-service → connector-gateway via gRPC/HTTP.

use async_trait::async_trait;
use uuid::Uuid;
use shared_types::Money;

use platform_error::PlatformError;

/// Request to authorize a payment via the connector gateway.
#[derive(Debug, Clone)]
pub struct ConnectorAuthorizeRequest {
    pub connector_id: String,
    pub payment_method_token: String,
    pub amount: Money,
    pub idempotency_key: String,
    pub merchant_reference: String,
    pub card_scheme: Option<String>,
}

/// Response from a connector authorization attempt.
#[derive(Debug, Clone)]
pub struct ConnectorAuthorizeResponse {
    pub status: String, // "approved" | "declined" | "requires_3ds" | "partial_approval"
    pub acquirer_reference: Option<String>,
    pub decline_reason: Option<String>,
    pub approved_amount: Option<Money>,
    pub latency_ms: u32,
}

/// Request to capture a payment via the connector gateway.
#[derive(Debug, Clone)]
pub struct ConnectorCaptureRequest {
    pub connector_id: String,
    pub acquirer_reference: String,
    pub amount: Option<Money>,
}

/// Response from a connector capture attempt.
#[derive(Debug, Clone)]
pub struct ConnectorCaptureResponse {
    pub success: bool,
    pub captured_amount: Option<Money>,
    pub acquirer_reference: String,
}

/// Request to void a payment via the connector gateway.
#[derive(Debug, Clone)]
pub struct ConnectorVoidRequest {
    pub connector_id: String,
    pub acquirer_reference: String,
}

/// Response from a connector void attempt.
#[derive(Debug, Clone)]
pub struct ConnectorVoidResponse {
    pub success: bool,
    pub status: String,
}

/// Request to refund a payment via the connector gateway.
#[derive(Debug, Clone)]
pub struct ConnectorRefundRequest {
    pub connector_id: String,
    pub acquirer_reference: String,
    pub amount: Money,
    pub reason: Option<String>,
}

/// Response from a connector refund attempt.
#[derive(Debug, Clone)]
pub struct ConnectorRefundResponse {
    pub success: bool,
    pub refund_reference: String,
    pub status: String,
}

/// Connector client trait — abstracts the connector gateway HTTP calls.
///
/// In production, this makes real HTTP calls to the connector-gateway service.
/// In tests, this can be mocked.
#[async_trait]
pub trait ConnectorClient: Send + Sync {
    /// Authorize a payment via the connector gateway.
    async fn authorize(&self, req: ConnectorAuthorizeRequest) -> Result<ConnectorAuthorizeResponse, PlatformError>;

    /// Capture a payment via the connector gateway.
    async fn capture(&self, req: ConnectorCaptureRequest) -> Result<ConnectorCaptureResponse, PlatformError>;

    /// Void a payment via the connector gateway.
    async fn void(&self, req: ConnectorVoidRequest) -> Result<ConnectorVoidResponse, PlatformError>;

    /// Refund a payment via the connector gateway.
    async fn refund(&self, req: ConnectorRefundRequest) -> Result<ConnectorRefundResponse, PlatformError>;
}

/// Production connector client — makes real HTTP calls to connector-gateway.
pub struct HttpConnectorClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl HttpConnectorClient {
    pub fn new(base_url: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, http_client }
    }
}

#[async_trait]
impl ConnectorClient for HttpConnectorClient {
    async fn authorize(&self, req: ConnectorAuthorizeRequest) -> Result<ConnectorAuthorizeResponse, PlatformError> {
        let body = serde_json::json!({
            "connector_id": req.connector_id,
            "payment_method_token": req.payment_method_token,
            "amount": {
                "amount_minor_units": req.amount.amount_minor_units,
                "currency_code": req.amount.currency.0,
            },
            "idempotency_key": req.idempotency_key,
            "merchant_reference": req.merchant_reference,
            "card_scheme": req.card_scheme,
        });

        let response = self.http_client
            .post(format!("{}/v1/connectors/authorize", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Connector gateway unreachable: {e}")))?;

        let status = response.status();
        let resp_body: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::Internal(format!("Failed to parse connector response: {e}")))?;

        if !status.is_success() {
            let error_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown connector error");
            return Err(PlatformError::Unavailable(format!(
                "Connector gateway returned {}: {}", status, error_msg
            )));
        }

        let latency_ms = resp_body["latency_ms"].as_u64().unwrap_or(0) as u32;

        Ok(ConnectorAuthorizeResponse {
            status: resp_body["status"].as_str().unwrap_or("unknown").to_string(),
            acquirer_reference: resp_body["acquirer_reference"].as_str().map(String::from),
            decline_reason: resp_body["decline_reason"].as_str().map(String::from),
            approved_amount: resp_body.get("approved_amount").and_then(|a| {
                Some(Money {
                    amount_minor_units: a["amount_minor_units"].as_i64()?,
                    currency: shared_types::CurrencyCode::new(
                        a["currency_code"].as_str()?
                    ).ok()?,
                })
            }),
            latency_ms,
        })
    }

    async fn capture(&self, req: ConnectorCaptureRequest) -> Result<ConnectorCaptureResponse, PlatformError> {
        let body = match &req.amount {
            Some(a) => serde_json::json!({
                "connector_id": req.connector_id,
                "acquirer_reference": req.acquirer_reference,
                "amount": { "amount_minor_units": a.amount_minor_units, "currency_code": a.currency.0 },
            }),
            None => serde_json::json!({
                "connector_id": req.connector_id,
                "acquirer_reference": req.acquirer_reference,
                "amount": null,
            }),
        };

        let response = self.http_client
            .post(format!("{}/v1/connectors/capture", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Connector gateway unreachable: {e}")))?;

        let status = response.status();
        let resp_body: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::Internal(format!("Failed to parse connector response: {e}")))?;

        if !status.is_success() {
            let error_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown connector error");
            return Err(PlatformError::Unavailable(format!(
                "Connector capture failed {}: {}", status, error_msg
            )));
        }

        Ok(ConnectorCaptureResponse {
            success: resp_body["success"].as_bool().unwrap_or(false),
            captured_amount: resp_body.get("captured_amount").and_then(|a| {
                Some(Money {
                    amount_minor_units: a["amount_minor_units"].as_i64()?,
                    currency: shared_types::CurrencyCode::new(
                        a["currency_code"].as_str()?
                    ).ok()?,
                })
            }),
            acquirer_reference: resp_body["acquirer_reference"]
                .as_str()
                .unwrap_or(&req.acquirer_reference)
                .to_string(),
        })
    }

    async fn void(&self, req: ConnectorVoidRequest) -> Result<ConnectorVoidResponse, PlatformError> {
        let body = serde_json::json!({
            "connector_id": req.connector_id,
            "acquirer_reference": req.acquirer_reference,
        });

        let response = self.http_client
            .post(format!("{}/v1/connectors/void", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Connector gateway unreachable: {e}")))?;

        let status = response.status();
        let resp_body: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::Internal(format!("Failed to parse connector response: {e}")))?;

        if !status.is_success() {
            let error_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown connector error");
            return Err(PlatformError::Unavailable(format!(
                "Connector void failed {}: {}", status, error_msg
            )));
        }

        Ok(ConnectorVoidResponse {
            success: resp_body["success"].as_bool().unwrap_or(false),
            status: resp_body["status"].as_str().unwrap_or("voided").to_string(),
        })
    }

    async fn refund(&self, req: ConnectorRefundRequest) -> Result<ConnectorRefundResponse, PlatformError> {
        let body = serde_json::json!({
            "connector_id": req.connector_id,
            "acquirer_reference": req.acquirer_reference,
            "amount": {
                "amount_minor_units": req.amount.amount_minor_units,
                "currency_code": req.amount.currency.0,
            },
            "reason": req.reason,
        });

        let response = self.http_client
            .post(format!("{}/v1/connectors/refund", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Connector gateway unreachable: {e}")))?;

        let status = response.status();
        let resp_body: serde_json::Value = response.json().await
            .map_err(|e| PlatformError::Internal(format!("Failed to parse connector response: {e}")))?;

        if !status.is_success() {
            let error_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown connector error");
            return Err(PlatformError::Unavailable(format!(
                "Connector refund failed {}: {}", status, error_msg
            )));
        }

        Ok(ConnectorRefundResponse {
            success: resp_body["success"].as_bool().unwrap_or(false),
            refund_reference: resp_body["refund_reference"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            status: resp_body["status"].as_str().unwrap_or("refunded").to_string(),
        })
    }
}

/// Mock connector client for testing — returns configurable responses.
pub struct MockConnectorClient {
    pub authorize_response: Result<ConnectorAuthorizeResponse, PlatformError>,
    pub capture_response: Result<ConnectorCaptureResponse, PlatformError>,
    pub void_response: Result<ConnectorVoidResponse, PlatformError>,
    pub refund_response: Result<ConnectorRefundResponse, PlatformError>,
    pub authorize_call_count: std::sync::atomic::AtomicU32,
}

impl MockConnectorClient {
    pub fn approve_all() -> Self {
        Self {
            authorize_response: Ok(ConnectorAuthorizeResponse {
                status: "approved".to_string(),
                acquirer_reference: Some(format!("acq_{}", Uuid::now_v7())),
                decline_reason: None,
                approved_amount: None,
                latency_ms: 100,
            }),
            capture_response: Ok(ConnectorCaptureResponse {
                success: true,
                captured_amount: None,
                acquirer_reference: format!("acq_{}", Uuid::now_v7()),
            }),
            void_response: Ok(ConnectorVoidResponse {
                success: true,
                status: "voided".to_string(),
            }),
            refund_response: Ok(ConnectorRefundResponse {
                success: true,
                refund_reference: format!("ref_{}", Uuid::now_v7()),
                status: "refunded".to_string(),
            }),
            authorize_call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn decline_first_then_approve() -> Self {
        Self {
            authorize_response: Ok(ConnectorAuthorizeResponse {
                status: "declined".to_string(),
                acquirer_reference: None,
                decline_reason: Some("insufficient_funds".to_string()),
                approved_amount: None,
                latency_ms: 50,
            }),
            capture_response: Ok(ConnectorCaptureResponse {
                success: true,
                captured_amount: None,
                acquirer_reference: format!("acq_{}", Uuid::now_v7()),
            }),
            void_response: Ok(ConnectorVoidResponse {
                success: true,
                status: "voided".to_string(),
            }),
            refund_response: Ok(ConnectorRefundResponse {
                success: true,
                refund_reference: format!("ref_{}", Uuid::now_v7()),
                status: "refunded".to_string(),
            }),
            authorize_call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl ConnectorClient for MockConnectorClient {
    async fn authorize(&self, _req: ConnectorAuthorizeRequest) -> Result<ConnectorAuthorizeResponse, PlatformError> {
        self.authorize_call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.authorize_response.clone()
    }

    async fn capture(&self, _req: ConnectorCaptureRequest) -> Result<ConnectorCaptureResponse, PlatformError> {
        self.capture_response.clone()
    }

    async fn void(&self, _req: ConnectorVoidRequest) -> Result<ConnectorVoidResponse, PlatformError> {
        self.void_response.clone()
    }

    async fn refund(&self, _req: ConnectorRefundRequest) -> Result<ConnectorRefundResponse, PlatformError> {
        self.refund_response.clone()
    }
}

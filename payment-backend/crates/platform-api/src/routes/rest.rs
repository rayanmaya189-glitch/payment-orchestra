//! REST API handlers for payment operations.
//!
//! Provides HTTP/JSON endpoints alongside the existing gRPC handlers.
//! All responses follow RFC 7807 Problem Details format for errors.

use serde::{Deserialize, Serialize};

use crate::http::ProblemDetail;

// ─── Request/Response Types ─────────────────────────────────────────────────

/// Create a payment intent request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentIntentRequest {
    /// Amount in minor units (cents/paisa).
    #[serde(rename = "amountMinor")]
    pub amount_minor: i64,
    /// Currency code (ISO 4217).
    pub currency: String,
    /// Idempotency key for deduplication.
    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,
    /// Payment purpose (Payment or CardVerification).
    #[serde(default = "default_purpose")]
    pub purpose: String,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

fn default_purpose() -> String {
    "Payment".to_string()
}

/// Payment intent response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentResponse {
    /// Payment intent ID.
    pub id: String,
    /// Current status.
    pub status: String,
    /// Requested amount.
    #[serde(rename = "amountMinor")]
    pub amount_minor: i64,
    /// Currency code.
    pub currency: String,
    /// Idempotency key.
    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,
    /// Created timestamp.
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Updated timestamp.
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// List payment intents query parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct ListPaymentIntentsQuery {
    /// Page number (1-based).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// Filter by status.
    pub status: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// Paginated list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// List of items.
    pub data: Vec<T>,
    /// Total count.
    pub total: u64,
    /// Current page.
    pub page: u32,
    /// Page size.
    pub page_size: u32,
    /// Total pages.
    pub total_pages: u32,
}

/// Capture payment intent request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturePaymentIntentRequest {
    /// Amount to capture (None = full authorized amount).
    #[serde(rename = "amountMinor")]
    pub amount_minor: Option<i64>,
}

/// Refund payment intent request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundPaymentIntentRequest {
    /// Amount to refund in minor units.
    #[serde(rename = "amountMinor")]
    pub amount_minor: i64,
    /// Optional reason for refund.
    pub reason: Option<String>,
}

/// Void payment intent request (no body needed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidPaymentIntentRequest {
    /// Optional reason for void.
    pub reason: Option<String>,
}

// ─── Error Conversion ───────────────────────────────────────────────────────

/// Generic error code to HTTP status mapping.
/// Used by services to convert their domain errors to ProblemDetail.
pub fn error_code_to_http_status(error_code: &str) -> (u16, &'static str) {
    match error_code {
        // 400 Bad Request
        "VALIDATION_ERROR" => (400, "Validation Error"),
        "IDEMPOTENCY_KEY_REQUIRED" => (400, "Idempotency Key Required"),
        
        // 401 Unauthorized
        "AUTHENTICATION_ERROR" => (401, "Authentication Error"),
        
        // 403 Forbidden
        "AUTHORIZATION_ERROR" => (403, "Authorization Error"),
        
        // 404 Not Found
        "PAYMENT_INTENT_NOT_FOUND" | "ROUTING_POLICY_NOT_FOUND" => {
            (404, "Not Found")
        }
        
        // 409 Conflict
        "CONCURRENCY_CONFLICT" | "IDEMPOTENCY_CONFLICT" => {
            (409, "Conflict")
        }
        
        // 422 Unprocessable Entity
        "PAYMENT_INTENT_NOT_AUTHORIZED"
        | "PAYMENT_INTENT_FAILED"
        | "PAYMENT_INTENT_VOIDED"
        | "AUTHORIZATION_EXPIRED"
        | "PAYMENT_INTENT_ALREADY_CAPTURED"
        | "PAYMENT_INTENT_FULLY_REFUNDED"
        | "PAYMENT_INTENT_AUTHORIZING"
        | "PAYMENT_INTENT_CAPTURING"
        | "NO_ELIGIBLE_ROUTE"
        | "ALL_ACQUIRERS_DECLINED"
        | "PAYMENT_METHOD_TOKEN_INVALID" => {
            (422, "Invalid State")
        }
        
        // 429 Rate Limited
        "RATE_LIMITED" => (429, "Rate Limited"),
        
        // 500 Internal Server Error
        _ => (500, "Internal Server Error"),
    }
}

/// Create a ProblemDetail from an error code and message.
pub fn problem_from_error_code(
    error_code: &str,
    message: &str,
    request_id: &str,
) -> ProblemDetail {
    let (status, title) = error_code_to_http_status(error_code);
    let problem_type = format!("https://api.payment-orchestra.com/errors/{}", error_code.to_lowercase());
    
    ProblemDetail::new(&problem_type, title, status, message)
        .with_request_id(request_id)
        .with_extension("errorCode".to_string(), serde_json::json!(error_code))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_payment_intent_request_deserialization() {
        // REST API uses camelCase JSON
        let json = r#"{
            "amountMinor": 10000,
            "currency": "AED",
            "idempotencyKey": "test-key-1234567890"
        }"#;
        let req: CreatePaymentIntentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount_minor, 10000);
        assert_eq!(req.currency, "AED");
        assert_eq!(req.purpose, "Payment"); // default
    }

    #[test]
    fn test_payment_intent_response_serialization() {
        let resp = PaymentIntentResponse {
            id: "test-id".to_string(),
            status: "Created".to_string(),
            amount_minor: 10000,
            currency: "AED".to_string(),
            idempotency_key: "test-key".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"amountMinor\":10000"));
        assert!(json.contains("\"idempotencyKey\":\"test-key\""));
    }

    #[test]
    fn test_paginated_response_serialization() {
        let resp: PaginatedResponse<PaymentIntentResponse> = PaginatedResponse {
            data: vec![],
            total: 0,
            page: 1,
            page_size: 20,
            total_pages: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"page\":1"));
    }
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HTTP response helpers.
pub fn protobuf_content_type() -> &'static str {
    "application/protobuf"
}

// ─── RFC 7807 Problem Details ───────────────────────────────────────────────

/// RFC 7807 Problem Details for HTTP APIs.
///
/// Standard error response format for REST APIs. Provides machine-readable
/// error types and human-readable descriptions.
///
/// See: https://www.rfc-editor.org/rfc/rfc7807
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetail {
    /// A URI reference that identifies the problem type.
    #[serde(rename = "type")]
    pub problem_type: String,

    /// A short, human-readable summary of the problem type.
    pub title: String,

    /// The HTTP status code.
    pub status: u16,

    /// A human-readable explanation specific to this occurrence.
    pub detail: String,

    /// A URI reference that identifies the specific occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Unique request ID for tracing.
    #[serde(rename = "requestId")]
    pub request_id: String,

    /// Additional extension members.
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl ProblemDetail {
    /// Create a new ProblemDetail.
    pub fn new(
        problem_type: impl Into<String>,
        title: impl Into<String>,
        status: u16,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            problem_type: problem_type.into(),
            title: title.into(),
            status,
            detail: detail.into(),
            instance: None,
            request_id: Uuid::now_v7().to_string(),
            extensions: serde_json::Map::new(),
        }
    }

    /// Set the instance URI.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Add an extension field.
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Get the extensions as a reference.
    pub fn extensions(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.extensions
    }

    /// Set the request ID.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = request_id.into();
        self
    }
}

// ─── Predefined Error Types ─────────────────────────────────────────────────

/// Standard problem type URIs.
pub mod problem_types {
    pub const VALIDATION_ERROR: &str = "https://api.payment-orchestra.com/errors/validation-error";
    pub const AUTHENTICATION_ERROR: &str = "https://api.payment-orchestra.com/errors/authentication-error";
    pub const AUTHORIZATION_ERROR: &str = "https://api.payment-orchestra.com/errors/authorization-error";
    pub const NOT_FOUND: &str = "https://api.payment-orchestra.com/errors/not-found";
    pub const RATE_LIMITED: &str = "https://api.payment-orchestra.com/errors/rate-limited";
    pub const IDEMPOTENCY_KEY_REQUIRED: &str = "https://api.payment-orchestra.com/errors/idempotency-key-required";
    pub const IDEMPOTENCY_CONFLICT: &str = "https://api.payment-orchestra.com/errors/idempotency-conflict";
    pub const PAYMENT_FAILED: &str = "https://api.payment-orchestra.com/errors/payment-failed";
    pub const CONNECTOR_ERROR: &str = "https://api.payment-orchestra.com/errors/connector-error";
    pub const INTERNAL_ERROR: &str = "https://api.payment-orchestra.com/errors/internal-error";
}

/// Convenience constructors for common error responses.
pub mod problems {
    use super::*;

    pub fn validation_error(detail: impl Into<String>, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::VALIDATION_ERROR,
            "Validation Error",
            400,
            detail,
        )
        .with_request_id(request_id)
    }

    pub fn authentication_error(detail: impl Into<String>, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::AUTHENTICATION_ERROR,
            "Authentication Error",
            401,
            detail,
        )
        .with_request_id(request_id)
    }

    pub fn authorization_error(detail: impl Into<String>, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::AUTHORIZATION_ERROR,
            "Authorization Error",
            403,
            detail,
        )
        .with_request_id(request_id)
    }

    pub fn not_found(resource: &str, id: &str, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::NOT_FOUND,
            "Not Found",
            404,
            format!("{} with id '{}' not found", resource, id),
        )
        .with_request_id(request_id)
    }

    pub fn rate_limited(retry_after_ms: u64, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::RATE_LIMITED,
            "Rate Limited",
            429,
            format!("Rate limit exceeded. Retry after {}ms", retry_after_ms),
        )
        .with_request_id(request_id)
        .with_extension(
            "retryAfterMs".to_string(),
            serde_json::json!(retry_after_ms),
        )
    }

    pub fn idempotency_key_required(request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::IDEMPOTENCY_KEY_REQUIRED,
            "Idempotency Key Required",
            400,
            "Idempotency-Key header is required for POST requests",
        )
        .with_request_id(request_id)
    }

    pub fn idempotency_conflict(
        original_request_id: &str,
        request_id: impl Into<String>,
    ) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::IDEMPOTENCY_CONFLICT,
            "Idempotency Conflict",
            409,
            format!(
                "A request with this idempotency key was already processed (requestId: {})",
                original_request_id
            ),
        )
        .with_request_id(request_id)
    }

    pub fn internal_error(detail: impl Into<String>, request_id: impl Into<String>) -> ProblemDetail {
        ProblemDetail::new(
            problem_types::INTERNAL_ERROR,
            "Internal Server Error",
            500,
            detail,
        )
        .with_request_id(request_id)
    }
}

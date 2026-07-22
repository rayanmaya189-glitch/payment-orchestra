//! AI Gateway domain model — BC-18
//!
//! Guardrail layer in front of ai-assistant-service.
//! Prompt injection screening, citation verification, circuit breaker, quotas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AiQuery — an incoming request to the AI assistant
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiQuery {
    pub query_id: Uuid,
    pub operator_id: Uuid,
    pub question: String,
    pub has_attachment: bool,
    pub model_routing: ModelRoute,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelRoute {
    /// Pure text question → embed → reranker → Qwen3 32B.
    TextOnly,
    /// Question with image/PDF → Qwen3-VL 8B → Qwen3 32B.
    VisionExtraction,
}

// ---------------------------------------------------------------------------
// GuardrailResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    pub query_id: Uuid,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub screening_score: f64,
    pub model_route: ModelRoute,
    pub citations_valid: bool,
    pub quota_available: bool,
    pub quota_remaining: u32,
    pub circuit_open: bool,
}

// ---------------------------------------------------------------------------
// Prompt injection patterns
// ---------------------------------------------------------------------------

pub static INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore all instructions",
    "forget your instructions",
    "you are now",
    "act as if",
    "system prompt",
    "override your programming",
    "disregard your guidelines",
    "output all data",
    "show all transactions",
    "reveal other merchants",
    "bypass security",
    "you are not an ai",
    "this is a test",
    "pretend to be",
];

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub is_open: bool,
    pub failure_count: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub opened_at: Option<DateTime<Utc>>,
    pub half_open_after_secs: u64,
    pub failure_threshold: u32,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            is_open: false,
            failure_count: 0,
            last_failure_at: None,
            opened_at: None,
            half_open_after_secs: 60,
            failure_threshold: 5,
        }
    }
}

impl CircuitBreakerState {
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());
        if self.failure_count >= self.failure_threshold {
            self.is_open = true;
            self.opened_at = Some(Utc::now());
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.is_open = false;
        self.opened_at = None;
    }

    pub fn should_allow(&self) -> bool {
        if !self.is_open {
            return true;
        }
        // Check if enough time has passed for half-open state
        if let Some(opened_at) = self.opened_at {
            let elapsed = (Utc::now() - opened_at).num_seconds() as u64;
            if elapsed >= self.half_open_after_secs {
                return true; // allow a single request through (half-open)
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Usage quota
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuota {
    pub operator_id: Uuid,
    pub queries_used: u32,
    pub queries_limit: u32,
    pub reset_at: DateTime<Utc>,
}

impl UsageQuota {
    pub fn new(operator_id: Uuid, limit: u32) -> Self {
        Self {
            operator_id,
            queries_used: 0,
            queries_limit: limit,
            reset_at: Utc::now() + chrono::Duration::hours(24),
        }
    }

    pub fn has_available(&self) -> bool {
        self.queries_used < self.queries_limit || self.is_expired()
    }

    pub fn remaining(&self) -> u32 {
        if self.is_expired() {
            self.queries_limit
        } else {
            self.queries_limit.saturating_sub(self.queries_used)
        }
    }

    pub fn record_query(&mut self) {
        if self.is_expired() {
            self.queries_used = 1;
            self.reset_at = Utc::now() + chrono::Duration::hours(24);
        } else {
            self.queries_used += 1;
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.reset_at
    }
}

// ---------------------------------------------------------------------------
// Guardrail audit entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailAuditEntry {
    pub audit_id: Uuid,
    pub query_id: Uuid,
    pub operator_id: Uuid,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub screening_score: f64,
    pub citations_valid: bool,
    pub circuit_open: bool,
    pub quota_remaining: u32,
    pub processed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum AiGatewayError {
    #[error("Query blocked by guardrail: {0}")]
    QueryBlocked(String),
    #[error("Circuit breaker open — AI service temporarily unavailable")]
    CircuitOpen,
    #[error("Usage quota exceeded")]
    QuotaExceeded,
    #[error("Invalid citation detected")]
    InvalidCitation,
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Model unavailable: {0}")]
    ModelUnavailable(String),
    #[error("Audit log error: {0}")]
    AuditError(String),
}

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::value_objects::{PromptCategory, SeverityLevel};

#[derive(Debug, Clone)]
pub struct GuardrailViolation {
    pub violation_id: Uuid,
    pub request_id: Uuid,
    pub category: PromptCategory,
    pub pattern_matched: String,
    pub severity: SeverityLevel,
    pub original_text: String,
    pub redacted_text: String,
    pub created_at: DateTime<Utc>,
}

impl GuardrailViolation {
    pub fn new(
        request_id: Uuid,
        category: PromptCategory,
        pattern_matched: String,
        severity: SeverityLevel,
        original_text: String,
        redacted_text: String,
    ) -> Self {
        Self {
            violation_id: Uuid::now_v7(),
            request_id,
            category,
            pattern_matched,
            severity,
            original_text,
            redacted_text,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    pub principal_id: Uuid,
    pub window_start: DateTime<Utc>,
    pub request_count: u32,
    pub max_requests: u32,
    pub window_seconds: u32,
}

impl RateLimitEntry {
    pub fn new(principal_id: Uuid, max_requests: u32, window_seconds: u32) -> Self {
        Self {
            principal_id,
            window_start: Utc::now(),
            request_count: 0,
            max_requests,
            window_seconds,
        }
    }

    pub fn is_within_limit(&self) -> bool {
        self.request_count < self.max_requests
    }

    pub fn remaining(&self) -> u32 {
        self.max_requests.saturating_sub(self.request_count)
    }

    pub fn seconds_until_reset(&self) -> i64 {
        let elapsed = Utc::now().signed_duration_since(self.window_start);
        let window = chrono::Duration::seconds(self.window_seconds as i64);
        (window - elapsed).num_seconds().max(0)
    }

    pub fn increment(&mut self) {
        self.request_count += 1;
    }
}

#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub request_id: Uuid,
    pub principal_id: Uuid,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub blocked: bool,
    pub created_at: DateTime<Utc>,
}

impl RequestMetrics {
    pub fn new(request_id: Uuid, principal_id: Uuid, model: String) -> Self {
        Self {
            request_id,
            principal_id,
            model,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_usd: 0.0,
            latency_ms: 0,
            blocked: false,
            created_at: Utc::now(),
        }
    }

    pub fn record_tokens(&mut self, input: u32, output: u32) {
        self.input_tokens = input;
        self.output_tokens = output;
        self.total_tokens = input + output;
    }

    pub fn record_cost(&mut self, cost: f64) {
        self.cost_usd = cost;
    }

    pub fn record_latency(&mut self, ms: u64) {
        self.latency_ms = ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardrail_violation_creation() {
        let v = GuardrailViolation::new(
            Uuid::now_v7(),
            PromptCategory::InjectionAttempt,
            "ignore previous".into(),
            SeverityLevel::High,
            "original".into(),
            "redacted".into(),
        );
        assert_eq!(v.category, PromptCategory::InjectionAttempt);
        assert_eq!(v.severity, SeverityLevel::High);
    }

    #[test]
    fn test_rate_limit_entry() {
        let mut entry = RateLimitEntry::new(Uuid::now_v7(), 10, 60);
        assert!(entry.is_within_limit());
        assert_eq!(entry.remaining(), 10);

        for _ in 0..10 {
            entry.increment();
        }
        assert!(!entry.is_within_limit());
        assert_eq!(entry.remaining(), 0);
    }

    #[test]
    fn test_request_metrics() {
        let mut metrics = RequestMetrics::new(Uuid::now_v7(), Uuid::now_v7(), "qwen3".into());
        metrics.record_tokens(100, 50);
        assert_eq!(metrics.total_tokens, 150);
        metrics.record_cost(0.00015);
        assert!((metrics.cost_usd - 0.00015).abs() < f64::EPSILON);
    }
}

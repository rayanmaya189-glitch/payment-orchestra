use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::value_objects::{ForwardStatus, RateLimitConfig};

#[derive(Debug, Clone)]
pub struct ForwardResult {
    pub request_id: Uuid,
    pub route_id: Option<Uuid>,
    pub method: String,
    pub path: String,
    pub target_url: String,
    pub target_service: String,
    pub status: ForwardStatus,
    pub response_status: Option<u16>,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ForwardResult {
    pub fn new(
        method: String,
        path: String,
        target_url: String,
        target_service: String,
    ) -> Self {
        Self {
            request_id: Uuid::now_v7(),
            route_id: None,
            method,
            path,
            target_url,
            target_service,
            status: ForwardStatus::Success,
            response_status: None,
            response_time_ms: 0,
            error_message: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_status(mut self, status: ForwardStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_response(mut self, status_code: u16, time_ms: u64) -> Self {
        self.response_status = Some(status_code);
        self.response_time_ms = time_ms;
        self
    }

    pub fn with_error(mut self, message: String) -> Self {
        self.error_message = Some(message);
        self
    }

    pub fn is_success(&self) -> bool {
        self.status == ForwardStatus::Success
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "request_id": self.request_id.to_string(),
            "method": self.method,
            "path": self.path,
            "target_service": self.target_service,
            "target_url": self.target_url,
            "status": self.status.label(),
            "response_status": self.response_status,
            "response_time_ms": self.response_time_ms,
            "error_message": self.error_message,
            "created_at": self.created_at.to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RouteRateLimitState {
    pub route_id: Uuid,
    pub path_prefix: String,
    pub principal_id: Uuid,
    pub request_count: u32,
    pub window_start: DateTime<Utc>,
    pub config: RateLimitConfig,
}

impl RouteRateLimitState {
    pub fn new(route_id: Uuid, path_prefix: String, principal_id: Uuid, config: RateLimitConfig) -> Self {
        Self {
            route_id,
            path_prefix,
            principal_id,
            request_count: 0,
            window_start: Utc::now(),
            config,
        }
    }

    pub fn is_within_limit(&self) -> bool {
        self.request_count < self.config.max_requests
    }

    pub fn remaining(&self) -> u32 {
        self.config.max_requests.saturating_sub(self.request_count)
    }

    pub fn seconds_until_reset(&self) -> i64 {
        let elapsed = Utc::now().signed_duration_since(self.window_start);
        let window = chrono::Duration::seconds(self.config.window_seconds as i64);
        (window - elapsed).num_seconds().max(0)
    }

    pub fn increment(&mut self) {
        self.request_count += 1;
    }

    pub fn should_reset(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.window_start);
        elapsed.num_seconds() >= self.config.window_seconds as i64
    }

    pub fn reset_if_needed(&mut self) {
        if self.should_reset() {
            self.request_count = 0;
            self.window_start = Utc::now();
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteHealth {
    pub route_id: Uuid,
    pub path_prefix: String,
    pub target_service: String,
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub total_forwards: u64,
}

impl RouteHealth {
    pub fn new(route_id: Uuid, path_prefix: String, target_service: String) -> Self {
        Self {
            route_id,
            path_prefix,
            target_service,
            is_healthy: true,
            last_check: Utc::now(),
            avg_response_time_ms: 0.0,
            success_rate: 100.0,
            total_forwards: 0,
        }
    }

    pub fn record_forward(&mut self, result: &ForwardResult) {
        self.total_forwards += 1;
        self.last_check = Utc::now();

        let count = self.total_forwards as f64;
        self.avg_response_time_ms =
            (self.avg_response_time_ms * (count - 1.0) + result.response_time_ms as f64) / count;

        if result.is_success() {
            let successes = (self.success_rate / 100.0 * (count - 1.0)) + 1.0;
            self.success_rate = (successes / count) * 100.0;
        } else {
            let successes = self.success_rate / 100.0 * (count - 1.0);
            self.success_rate = (successes / count) * 100.0;
        }

        self.is_healthy = self.success_rate >= 95.0;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "route_id": self.route_id.to_string(),
            "path_prefix": self.path_prefix,
            "target_service": self.target_service,
            "is_healthy": self.is_healthy,
            "avg_response_time_ms": self.avg_response_time_ms,
            "success_rate": self.success_rate,
            "total_forwards": self.total_forwards,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_result() {
        let result = ForwardResult::new("GET".into(), "/v1/payments".into(), "http://localhost:8081".into(), "payment-service".into())
            .with_status(ForwardStatus::Success)
            .with_response(200, 45);
        assert!(result.is_success());
        assert_eq!(result.response_time_ms, 45);
    }

    #[test]
    fn test_route_rate_limit() {
        let mut state = RouteRateLimitState::new(
            Uuid::now_v7(),
            "/v1/payments".into(),
            Uuid::now_v7(),
            RateLimitConfig::new(5, 60).unwrap(),
        );
        assert!(state.is_within_limit());
        assert_eq!(state.remaining(), 5);

        for _ in 0..5 {
            state.increment();
        }
        assert!(!state.is_within_limit());
        assert_eq!(state.remaining(), 0);
    }

    #[test]
    fn test_route_health() {
        let mut health = RouteHealth::new(Uuid::now_v7(), "/v1".into(), "test".into());
        let result = ForwardResult::new("GET".into(), "/test".into(), "http://test".into(), "test".into())
            .with_status(ForwardStatus::Success)
            .with_response(200, 50);

        health.record_forward(&result);
        assert_eq!(health.total_forwards, 1);
        assert!(health.is_healthy);
    }
}

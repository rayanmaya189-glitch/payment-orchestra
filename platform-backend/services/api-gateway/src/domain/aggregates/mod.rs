use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::value_objects::RateLimitConfig;

#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub auth_required: bool,
    pub rate_limit: Option<RateLimitConfig>,
    pub methods: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub retry_count: Option<u32>,
    pub health_check_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RouteConfig {
    pub fn new(path_prefix: String, target_service: String, target_url: String) -> Self {
        let now = Utc::now();
        Self {
            path_prefix,
            target_service,
            target_url,
            auth_required: true,
            rate_limit: None,
            methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
            timeout_ms: Some(30_000),
            retry_count: Some(2),
            health_check_path: Some("/healthz".into()),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_auth_required(mut self, required: bool) -> Self {
        self.auth_required = required;
        self
    }

    pub fn with_rate_limit(mut self, max_requests: u32, window_seconds: u32) -> Self {
        self.rate_limit = Some(RateLimitConfig::new(max_requests, window_seconds).unwrap());
        self
    }

    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn supports_method(&self, method: &str) -> bool {
        self.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
    }

    pub fn matches_path(&self, path: &str) -> bool {
        path.starts_with(&self.path_prefix)
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path_prefix": self.path_prefix,
            "target_service": self.target_service,
            "target_url": self.target_url,
            "auth_required": self.auth_required,
            "rate_limit": self.rate_limit.as_ref().map(|rl| serde_json::json!({
                "max_requests": rl.max_requests,
                "window_seconds": rl.window_seconds,
            })),
            "methods": self.methods,
            "timeout_ms": self.timeout_ms,
            "retry_count": self.retry_count,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
    pub principal_id: Option<String>,
    pub query_string: Option<String>,
    pub request_id: Option<String>,
}

impl GatewayRequest {
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            headers: std::collections::HashMap::new(),
            body: vec![],
            principal_id: None,
            query_string: None,
            request_id: None,
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn with_principal(mut self, principal_id: String) -> Self {
        self.principal_id = Some(principal_id);
        self
    }

    pub fn full_path(&self) -> String {
        match &self.query_string {
            Some(q) => format!("{}?{}", self.path, q),
            None => self.path.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
}

impl GatewayResponse {
    pub fn ok() -> Self {
        Self {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: vec![],
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: std::collections::HashMap::new(),
            body: serde_json::to_vec(&serde_json::json!({"error": "Not found"})).unwrap(),
        }
    }

    pub fn too_many_requests(retry_after: u64) -> Self {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Retry-After".into(), retry_after.to_string());
        Self {
            status: 429,
            headers,
            body: serde_json::to_vec(&serde_json::json!({
                "error": "Too many requests",
                "retry_after_seconds": retry_after,
            }))
            .unwrap(),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            status: 500,
            headers: std::collections::HashMap::new(),
            body: serde_json::to_vec(&serde_json::json!({"error": message})).unwrap(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

#[derive(Debug, Clone)]
pub struct GatewayStats {
    pub total_requests: u64,
    pub successful_forwards: u64,
    pub failed_forwards: u64,
    pub rate_limited: u64,
    pub not_found: u64,
    pub avg_response_time_ms: f64,
    pub active_routes: u64,
}

impl GatewayStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_forwards: 0,
            failed_forwards: 0,
            rate_limited: 0,
            not_found: 0,
            avg_response_time_ms: 0.0,
            active_routes: 0,
        }
    }

    pub fn record_request(&mut self, response_time_ms: u64, success: bool) {
        self.total_requests += 1;
        if success {
            self.successful_forwards += 1;
        } else {
            self.failed_forwards += 1;
        }
        let count = self.total_requests as f64;
        self.avg_response_time_ms =
            (self.avg_response_time_ms * (count - 1.0) + response_time_ms as f64) / count;
    }

    pub fn record_rate_limited(&mut self) {
        self.rate_limited += 1;
    }

    pub fn record_not_found(&mut self) {
        self.not_found += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.successful_forwards as f64 / self.total_requests as f64) * 100.0
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_requests": self.total_requests,
            "successful_forwards": self.successful_forwards,
            "failed_forwards": self.failed_forwards,
            "rate_limited": self.rate_limited,
            "not_found": self.not_found,
            "success_rate": self.success_rate(),
            "avg_response_time_ms": self.avg_response_time_ms,
            "active_routes": self.active_routes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_config() {
        let route = RouteConfig::new("/v1/payments".into(), "payment".into(), "http://localhost:8080".into())
            .with_auth_required(true)
            .with_rate_limit(100, 60);
        assert!(route.auth_required);
        assert!(route.rate_limit.is_some());
        assert!(route.supports_method("GET"));
        assert!(route.matches_path("/v1/payments/123"));
    }

    #[test]
    fn test_gateway_request() {
        let req = GatewayRequest::new("POST".into(), "/v1/payments".into())
            .with_principal("user-123".into());
        assert_eq!(req.method, "POST");
        assert_eq!(req.principal_id.unwrap(), "user-123");
    }

    #[test]
    fn test_gateway_response() {
        assert!(GatewayResponse::ok().is_success());
        assert!(!GatewayResponse::not_found().is_success());
        assert!(!GatewayResponse::internal_error("fail").is_success());
    }

    #[test]
    fn test_gateway_stats() {
        let mut stats = GatewayStats::new();
        stats.record_request(50, true);
        stats.record_request(100, false);
        assert_eq!(stats.total_requests, 2);
        assert!((stats.success_rate() - 50.0).abs() < f64::EPSILON);
    }
}

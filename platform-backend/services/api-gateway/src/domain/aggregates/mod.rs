use chrono::{DateTime, Utc};
use uuid::Uuid;

/// API Gateway route configuration — maps paths to downstream services.
#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub route_id: Uuid,
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub requires_auth: bool,
    pub rate_limit_per_second: u32,
    pub timeout_ms: u32,
    pub created_at: DateTime<Utc>,
}

impl RouteConfig {
    pub fn new(
        path_prefix: String,
        target_service: String,
        target_url: String,
        requires_auth: bool,
        rate_limit_per_second: u32,
    ) -> Self {
        Self {
            route_id: Uuid::now_v7(),
            path_prefix,
            target_service,
            target_url,
            requires_auth,
            rate_limit_per_second,
            timeout_ms: 30_000,
            created_at: Utc::now(),
        }
    }

    /// Check if this route matches a given path.
    pub fn matches_path(&self, path: &str) -> bool {
        path.starts_with(&self.path_prefix)
    }
}

/// API Gateway request context — extracted from incoming request.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub api_key_id: Option<String>,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub client_ip: String,
    pub user_agent: String,
    pub received_at: DateTime<Utc>,
}

impl RequestContext {
    pub fn new(method: String, path: String, client_ip: String, user_agent: String) -> Self {
        Self {
            request_id: Uuid::now_v7(),
            principal_id: None,
            api_key_id: None,
            method,
            path,
            headers: Vec::new(),
            body: Vec::new(),
            client_ip,
            user_agent,
            received_at: Utc::now(),
        }
    }
}

/// API Gateway response — returned to the client.
#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub latency_ms: u64,
}

impl GatewayResponse {
    pub fn ok(body: Vec<u8>) -> Self {
        Self {
            status_code: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body,
            latency_ms: 0,
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status_code: status,
            headers: vec![("content-type".into(), "application/json".into())],
            body: serde_json::json!({"error": message, "code": status}).to_string().into_bytes(),
            latency_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_matches_path() {
        let route = RouteConfig::new(
            "/v1/payments".into(),
            "orchestration-service".into(),
            "http://localhost:8081".into(),
            true,
            100,
        );
        assert!(route.matches_path("/v1/payments/intents"));
        assert!(!route.matches_path("/v1/invoices"));
    }

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("GET".into(), "/v1/test".into(), "127.0.0.1".into(), "test-agent".into());
        assert_eq!(ctx.method, "GET");
        assert!(ctx.principal_id.is_none());
    }

    #[test]
    fn test_gateway_response_ok() {
        let resp = GatewayResponse::ok(b"{}".to_vec());
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn test_gateway_response_error() {
        let resp = GatewayResponse::error(404, "Not found");
        assert_eq!(resp.status_code, 404);
        let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(body["error"], "Not found");
    }
}

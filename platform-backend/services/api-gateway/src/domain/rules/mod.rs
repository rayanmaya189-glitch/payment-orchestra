use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{GatewayRequest, GatewayResponse, RouteConfig};
use crate::domain::entities::ForwardResult;
use crate::domain::value_objects::RateLimitConfig;
use platform_error::PlatformError;

#[async_trait]
pub trait RouteRepository: Send + Sync {
    async fn find_route(&self, path: &str) -> Result<Option<RouteConfig>, PlatformError>;
    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError>;
    async fn find_route_by_id(&self, id: Uuid) -> Result<Option<RouteConfig>, PlatformError>;
    async fn create_route(&self, route: &RouteConfig) -> Result<RouteConfig, PlatformError>;
    async fn update_route(&self, id: Uuid, route: &RouteConfig) -> Result<(), PlatformError>;
    async fn delete_route(&self, id: Uuid) -> Result<(), PlatformError>;
}

#[async_trait]
pub trait RequestForwarder: Send + Sync {
    async fn forward(&self, request: &GatewayRequest, target_url: &str) -> Result<GatewayResponse, PlatformError>;
    fn log_forward(&self, result: &ForwardResult);
}

#[async_trait]
pub trait RouteMatcher: Send + Sync {
    fn match_route<'a>(&self, path: &str, routes: &'a [RouteConfig]) -> Option<&'a RouteConfig>;
    fn extract_path_params(&self, pattern: &str, path: &str) -> std::collections::HashMap<String, String>;
}

pub struct DefaultRouteMatcher;

impl DefaultRouteMatcher {
    pub fn new() -> Self {
        Self
    }
}

impl RouteMatcher for DefaultRouteMatcher {
    fn match_route<'a>(&self, path: &str, routes: &'a [RouteConfig]) -> Option<&'a RouteConfig> {
        // Sort routes by specificity (longer prefix = more specific)
        let mut sorted_routes: Vec<&RouteConfig> = routes.iter().collect();
        sorted_routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));

        for route in sorted_routes {
            if path.starts_with(&route.path_prefix) {
                return Some(route);
            }
        }
        None
    }

    fn extract_path_params(&self, pattern: &str, path: &str) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        let pattern_parts: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
        let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        for (i, part) in pattern_parts.iter().enumerate() {
            if part.starts_with('{') && part.ends_with('}') {
                let param_name = &part[1..part.len() - 1];
                if i < path_parts.len() {
                    params.insert(param_name.to_string(), path_parts[i].to_string());
                }
            }
        }

        params
    }
}

pub struct StubForwarder;

impl StubForwarder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RequestForwarder for StubForwarder {
    async fn forward(
        &self,
        request: &GatewayRequest,
        target_url: &str,
    ) -> Result<GatewayResponse, PlatformError> {
        tracing::info!(
            method = %request.method,
            path = %request.path,
            target = %target_url,
            "Forwarding request (stub)"
        );

        // In production: actual HTTP forwarding with reqwest/hyper
        Ok(GatewayResponse {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: b"{}".to_vec(),
        })
    }

    fn log_forward(&self, result: &ForwardResult) {
        tracing::info!(
            request_id = %result.request_id,
            method = %result.method,
            path = %result.path,
            target = %result.target_service,
            status = %result.status.label(),
            response_ms = result.response_time_ms,
            "Request forwarded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_route(prefix: &str, service: &str) -> RouteConfig {
        RouteConfig {
            path_prefix: prefix.to_string(),
            target_service: service.to_string(),
            target_url: format!("http://localhost:{}", 8080),
            auth_required: true,
            rate_limit: None,
            methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
            timeout_ms: Some(30_000),
            retry_count: Some(2),
            health_check_path: Some("/healthz".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_route_matching() {
        let matcher = DefaultRouteMatcher::new();
        let routes = vec![
            make_route("/v1/payments", "payment-service"),
            make_route("/v1/operators", "operator-service"),
            make_route("/v1", "api-gateway"),
        ];

        let matched = matcher.match_route("/v1/payments/123", &routes);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().target_service, "payment-service");

        let matched = matcher.match_route("/v1/operators/456", &routes);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().target_service, "operator-service");

        let matched = matcher.match_route("/v1/other", &routes);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().target_service, "api-gateway");

        let matched = matcher.match_route("/v2/payments", &routes);
        assert!(matched.is_none());
    }

    #[test]
    fn test_path_param_extraction() {
        let matcher = DefaultRouteMatcher::new();
        let params = matcher.extract_path_params("/v1/payments/{id}", "/v1/payments/abc-123");
        assert_eq!(params.get("id").unwrap(), "abc-123");
    }

    #[tokio::test]
    async fn test_stub_forwarder() {
        let forwarder = StubForwarder::new();
        let request = GatewayRequest {
            method: "GET".into(),
            path: "/v1/test".into(),
            headers: std::collections::HashMap::new(),
            body: vec![],
            principal_id: None,
            query_string: None,
            request_id: None,
        };
        let result = forwarder.forward(&request, "http://localhost:8080").await;
        assert!(result.is_ok());
    }
}

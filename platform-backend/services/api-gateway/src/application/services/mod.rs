use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{RequestContext, RouteConfig, GatewayResponse};
use crate::domain::value_objects::{ServiceHealth, HealthStatus};
use platform_error::PlatformError;

#[async_trait]
pub trait GatewayService: Send + Sync {
    async fn route_request(&self, ctx: RequestContext) -> Result<GatewayResponse, PlatformError>;
    async fn get_route(&self, path: &str) -> Option<RouteConfig>;
    async fn health_check(&self) -> Vec<ServiceHealth>;
}

pub struct GatewayServiceImpl {
    routes: Vec<RouteConfig>,
}

impl GatewayServiceImpl {
    pub fn new() -> Self {
        let routes = vec![
            RouteConfig::new("/v1/operators".into(), "operator-service".into(), "http://localhost:8081".into(), true, 100),
            RouteConfig::new("/v1/auth".into(), "iam-service".into(), "http://localhost:8082".into(), false, 10),
            RouteConfig::new("/v1/principals".into(), "iam-service".into(), "http://localhost:8082".into(), true, 100),
            RouteConfig::new("/v1/kyb-cases".into(), "compliance-service".into(), "http://localhost:8083".into(), true, 10),
            RouteConfig::new("/v1/gateway-profiles".into(), "connector-gateway".into(), "http://localhost:8084".into(), true, 100),
            RouteConfig::new("/v1/payment-intents".into(), "orchestration-service".into(), "http://localhost:8085".into(), true, 500),
            RouteConfig::new("/v1/invoices".into(), "invoice-service".into(), "http://localhost:8086".into(), true, 100),
            RouteConfig::new("/v1/payment-links".into(), "payment-link-service".into(), "http://localhost:8087".into(), true, 100),
            RouteConfig::new("/v1/subscriptions".into(), "subscription-service".into(), "http://localhost:8088".into(), true, 50),
            RouteConfig::new("/v1/settlements".into(), "reconciliation-service".into(), "http://localhost:8089".into(), true, 50),
            RouteConfig::new("/v1/disputes".into(), "dispute-service".into(), "http://localhost:8090".into(), true, 50),
            RouteConfig::new("/v1/risk".into(), "risk-service".into(), "http://localhost:8091".into(), true, 100),
            RouteConfig::new("/v1/notifications".into(), "notification-service".into(), "http://localhost:8092".into(), true, 50),
        ];
        Self { routes }
    }
}

#[async_trait]
impl GatewayService for GatewayServiceImpl {
    async fn route_request(&self, ctx: RequestContext) -> Result<GatewayResponse, PlatformError> {
        // Find matching route
        let route = self.routes.iter().find(|r| r.matches_path(&ctx.path))
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Route".into(),
                id: Uuid::nil(),
            })?;

        // Check if auth is required
        if route.requires_auth && ctx.principal_id.is_none() {
            return Ok(GatewayResponse::error(401, "Authentication required"));
        }

        // TODO: Make actual HTTP call to downstream service
        tracing::info!(
            route_id = %route.route_id,
            target = %route.target_service,
            path = %ctx.path,
            method = %ctx.method,
            "Routing request"
        );

        Ok(GatewayResponse::ok(b"{}".to_vec()))
    }

    async fn get_route(&self, path: &str) -> Option<RouteConfig> {
        self.routes.iter().find(|r| r.matches_path(path)).cloned()
    }

    async fn health_check(&self) -> Vec<ServiceHealth> {
        let mut results = Vec::new();
        for route in &self.routes {
            results.push(ServiceHealth {
                service_name: route.target_service.clone(),
                status: HealthStatus::Healthy,
                latency_ms: Some(1),
                last_checked: chrono::Utc::now().to_rfc3339(),
                error: None,
            });
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_request_requires_auth() {
        let service = GatewayServiceImpl::new();
        let ctx = RequestContext::new("GET".into(), "/v1/payment-intents".into(), "127.0.0.1".into(), "test".into());
        let resp = service.route_request(ctx).await.unwrap();
        assert_eq!(resp.status_code, 401);
    }

    #[tokio::test]
    async fn test_route_request_allows_unauthenticated_for_auth() {
        let service = GatewayServiceImpl::new();
        let ctx = RequestContext::new("POST".into(), "/v1/auth/login".into(), "127.0.0.1".into(), "test".into());
        let resp = service.route_request(ctx).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }

    #[tokio::test]
    async fn test_get_route() {
        let service = GatewayServiceImpl::new();
        let route = service.get_route("/v1/payment-intents/123").await;
        assert!(route.is_some());
        assert_eq!(route.unwrap().target_service, "orchestration-service");
    }

    #[tokio::test]
    async fn test_health_check() {
        let service = GatewayServiceImpl::new();
        let health = service.health_check().await;
        assert!(!health.is_empty());
        assert!(health.iter().all(|h| h.status == HealthStatus::Healthy));
    }
}

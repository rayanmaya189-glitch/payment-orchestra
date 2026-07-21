use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::RouteConfig;
use crate::domain::rules::RouteRepository;
use crate::domain::value_objects::RateLimitConfig;
use platform_error::PlatformError;

pub struct NoopRouteRepository;

impl NoopRouteRepository {
    pub fn new() -> Self {
        Self
    }

    fn default_routes() -> Vec<RouteConfig> {
        let now = chrono::Utc::now();
        vec![
            RouteConfig {
                path_prefix: "/v1/payments".into(),
                target_service: "payment-service".into(),
                target_url: "http://payment-service:8080".into(),
                auth_required: true,
                rate_limit: Some(RateLimitConfig::new(1000, 60).unwrap()),
                methods: vec!["GET".into(), "POST".into()],
                timeout_ms: Some(30_000),
                retry_count: Some(2),
                health_check_path: Some("/healthz".into()),
                created_at: now,
                updated_at: now,
            },
            RouteConfig {
                path_prefix: "/v1/operators".into(),
                target_service: "operator-service".into(),
                target_url: "http://operator-service:8081".into(),
                auth_required: true,
                rate_limit: Some(RateLimitConfig::new(500, 60).unwrap()),
                methods: vec!["GET".into(), "POST".into(), "PUT".into()],
                timeout_ms: Some(30_000),
                retry_count: Some(2),
                health_check_path: Some("/healthz".into()),
                created_at: now,
                updated_at: now,
            },
            RouteConfig {
                path_prefix: "/v1".into(),
                target_service: "api-gateway".into(),
                target_url: "http://localhost:8080".into(),
                auth_required: true,
                rate_limit: Some(RateLimitConfig::new(2000, 60).unwrap()),
                methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
                timeout_ms: Some(30_000),
                retry_count: Some(2),
                health_check_path: Some("/healthz".into()),
                created_at: now,
                updated_at: now,
            },
        ]
    }
}

#[async_trait]
impl RouteRepository for NoopRouteRepository {
    async fn find_route(&self, path: &str) -> Result<Option<RouteConfig>, PlatformError> {
        let routes = Self::default_routes();
        let mut sorted: Vec<&RouteConfig> = routes.iter().collect();
        sorted.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));

        for route in sorted {
            if path.starts_with(&route.path_prefix) {
                return Ok(Some(route.clone()));
            }
        }
        Ok(None)
    }

    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError> {
        Ok(Self::default_routes())
    }

    async fn find_route_by_id(&self, _id: Uuid) -> Result<Option<RouteConfig>, PlatformError> {
        Ok(None)
    }

    async fn create_route(&self, route: &RouteConfig) -> Result<RouteConfig, PlatformError> {
        Ok(route.clone())
    }

    async fn update_route(&self, _id: Uuid, _route: &RouteConfig) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn delete_route(&self, _id: Uuid) -> Result<(), PlatformError> {
        Ok(())
    }
}

use async_trait::async_trait;
use crate::domain::aggregates::RouteConfig;
use crate::domain::rules::RouteRepository;
use platform_error::PlatformError;

pub struct NoopRouteRepository;
impl NoopRouteRepository { pub fn new() -> Self { Self } }

#[async_trait]
impl RouteRepository for NoopRouteRepository {
    async fn find_route(&self, _path: &str) -> Result<Option<RouteConfig>, PlatformError> {
        // Default routes — production loads from DB
        Ok(Some(RouteConfig {
            path_prefix: "/v1".to_string(), target_service: "api-gateway".to_string(),
            target_url: "http://localhost:8080".to_string(), auth_required: true, rate_limit: Some(1000),
        }))
    }
    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError> { Ok(vec![]) }
}

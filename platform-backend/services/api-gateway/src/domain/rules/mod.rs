use async_trait::async_trait;
use crate::domain::aggregates::RouteConfig;
use platform_error::PlatformError;

#[async_trait]
pub trait RouteRepository: Send + Sync {
    async fn find_route(&self, path: &str) -> Result<Option<RouteConfig>, PlatformError>;
    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError>;
}

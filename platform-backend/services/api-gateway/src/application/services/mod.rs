use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use crate::application::commands::*;
use crate::domain::aggregates::RouteConfig;
use crate::domain::rules::RouteRepository;
use platform_error::PlatformError;

pub struct ApiGatewayServiceImpl { repo: Box<dyn RouteRepository>, db: DatabaseConnection }
impl ApiGatewayServiceImpl { pub fn new(repo: Box<dyn RouteRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait ApiGatewayService: Send + Sync {
    async fn resolve_route(&self, cmd: RouteRequest) -> Result<RouteConfig, PlatformError>;
    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError>;
}

#[async_trait]
impl ApiGatewayService for ApiGatewayServiceImpl {
    async fn resolve_route(&self, cmd: RouteRequest) -> Result<RouteConfig, PlatformError> {
        self.repo.find_route(&cmd.path).await?.ok_or_else(|| PlatformError::NotFound { resource: "route".into(), id: uuid::Uuid::nil() })
    }
    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError> {
        self.repo.list_routes().await
    }
}

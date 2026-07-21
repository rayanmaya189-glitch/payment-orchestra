use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait};
use crate::domain::aggregates::RouteConfig;
use crate::domain::rules::RouteRepository;
use crate::infrastructure::entities::route_config_entity;
use platform_error::PlatformError;

pub struct PostgresRouteRepository { db: DatabaseConnection }
impl PostgresRouteRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl RouteRepository for PostgresRouteRepository {
    async fn find_route(&self, path: &str) -> Result<Option<RouteConfig>, PlatformError> {
        let models = route_config_entity::Entity::find().all(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        // Find matching route by prefix
        for model in models {
            if path.starts_with(&model.path_prefix) {
                return Ok(Some(RouteConfig {
                    path_prefix: model.path_prefix,
                    target_service: model.target_service,
                    target_url: model.target_url,
                    auth_required: model.auth_required,
                    rate_limit: model.rate_limit.map(|r| r as u32),
                }));
            }
        }
        Ok(None)
    }

    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError> {
        let models = route_config_entity::Entity::find().all(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(models.into_iter().map(|m| RouteConfig {
            path_prefix: m.path_prefix,
            target_service: m.target_service,
            target_url: m.target_url,
            auth_required: m.auth_required,
            rate_limit: m.rate_limit.map(|r| r as u32),
        }).collect())
    }
}

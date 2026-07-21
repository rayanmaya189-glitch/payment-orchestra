use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::domain::aggregates::RouteConfig;
use crate::domain::rules::RouteRepository;
use crate::domain::value_objects::RateLimitConfig;
use crate::infrastructure::entities::route_config_entity;
use platform_error::PlatformError;

pub struct PostgresRouteRepository {
    db: DatabaseConnection,
}

impl PostgresRouteRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RouteRepository for PostgresRouteRepository {
    async fn find_route(&self, path: &str) -> Result<Option<RouteConfig>, PlatformError> {
        let models = route_config_entity::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        let mut candidates: Vec<&route_config_entity::Model> = models
            .iter()
            .filter(|m| path.starts_with(&m.path_prefix))
            .collect();
        candidates.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));

        Ok(candidates.first().map(|m| RouteConfig {
            path_prefix: m.path_prefix.clone(),
            target_service: m.target_service.clone(),
            target_url: m.target_url.clone(),
            auth_required: m.auth_required,
            rate_limit: m.rate_limit.map(|r| RateLimitConfig::new(r as u32, 60).unwrap()),
            methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
            timeout_ms: Some(30_000),
            retry_count: Some(2),
            health_check_path: Some("/healthz".into()),
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }))
    }

    async fn list_routes(&self) -> Result<Vec<RouteConfig>, PlatformError> {
        let models = route_config_entity::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(models
            .into_iter()
            .map(|m| RouteConfig {
                path_prefix: m.path_prefix,
                target_service: m.target_service,
                target_url: m.target_url,
                auth_required: m.auth_required,
                rate_limit: m.rate_limit.map(|r| RateLimitConfig::new(r as u32, 60).unwrap()),
                methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
                timeout_ms: Some(30_000),
                retry_count: Some(2),
                health_check_path: Some("/healthz".into()),
                created_at: m.created_at.into(),
                updated_at: m.updated_at.into(),
            })
            .collect())
    }

    async fn find_route_by_id(&self, id: Uuid) -> Result<Option<RouteConfig>, PlatformError> {
        let model = route_config_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(model.map(|m| RouteConfig {
            path_prefix: m.path_prefix,
            target_service: m.target_service,
            target_url: m.target_url,
            auth_required: m.auth_required,
            rate_limit: m.rate_limit.map(|r| RateLimitConfig::new(r as u32, 60).unwrap()),
            methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
            timeout_ms: Some(30_000),
            retry_count: Some(2),
            health_check_path: Some("/healthz".into()),
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }))
    }

    async fn create_route(&self, route: &RouteConfig) -> Result<RouteConfig, PlatformError> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let a = route_config_entity::ActiveModel {
            route_id: Set(id),
            path_prefix: Set(route.path_prefix.clone()),
            target_service: Set(route.target_service.clone()),
            target_url: Set(route.target_url.clone()),
            auth_required: Set(route.auth_required),
            rate_limit: Set(route.rate_limit.as_ref().map(|rl| rl.max_requests as i32)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        a.insert(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(RouteConfig {
            path_prefix: route.path_prefix.clone(),
            target_service: route.target_service.clone(),
            target_url: route.target_url.clone(),
            auth_required: route.auth_required,
            rate_limit: route.rate_limit.clone(),
            methods: route.methods.clone(),
            timeout_ms: route.timeout_ms,
            retry_count: route.retry_count,
            health_check_path: route.health_check_path.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_route(&self, id: Uuid, route: &RouteConfig) -> Result<(), PlatformError> {
        let existing = route_config_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "route".into(),
                id,
            })?;

        let mut a = route_config_entity::ActiveModel::from(existing);
        a.path_prefix = Set(route.path_prefix.clone());
        a.target_service = Set(route.target_service.clone());
        a.target_url = Set(route.target_url.clone());
        a.auth_required = Set(route.auth_required);
        a.rate_limit = Set(route.rate_limit.as_ref().map(|rl| rl.max_requests as i32));
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(())
    }

    async fn delete_route(&self, id: Uuid) -> Result<(), PlatformError> {
        let existing = route_config_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "route".into(),
                id,
            })?;

        let active = route_config_entity::ActiveModel::from(existing);
        active
            .delete(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        Ok(())
    }
}

//! PostgreSQL-backed GatewayRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::GatewayRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as RouteActiveModel,
    Column as RouteColumn,
    Entity as RouteEntity,
    Model as RouteModel,
};

pub struct PostgresGatewayRepository {
    pub db: DatabaseConnection,
}

impl PostgresGatewayRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GatewayRepository for PostgresGatewayRepository {
    async fn find_route(&self, path: &str, method: &str) -> Result<Option<RouteDefinition>, String> {
        let result = RouteEntity::find()
            .filter(RouteColumn::Path.eq(path))
            .filter(RouteColumn::Method.eq(method))
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m))),
            None => Ok(None),
        }
    }

    async fn save_route(&self, route: &RouteDefinition) -> Result<(), String> {
        let model = domain_to_model(route);
        let exists = RouteEntity::find_by_id(route.route_id)
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?
            .is_some();

        if exists {
            RouteEntity::update(RouteActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            RouteEntity::insert(RouteActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, String> {
        let models = RouteEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn check_rate_limit(&self, _route_id: Uuid, _current_count: u32) -> Result<bool, String> {
        Ok(true) // Rate limiting handled at middleware level
    }

    async fn validate_api_key(&self, _key_hash: &str) -> Result<bool, String> {
        Ok(true) // API key validation handled by iam-service
    }

    async fn log_request(&self, _log_entry: &ApiRequestLog) -> Result<(), String> {
        Ok(()) // Request logging handled by middleware
    }

    async fn get_request_logs(&self, _route_id: Uuid, _since: DateTime<Utc>) -> Result<Vec<ApiRequestLog>, String> {
        Ok(vec![]) // Request logging handled by middleware
    }
}

fn domain_to_model(r: &RouteDefinition) -> RouteModel {
    RouteModel {
        route_id: r.route_id,
        path: r.path.clone(),
        method: r.method.clone(),
        grpc_service: r.grpc_service.clone(),
        grpc_method: r.grpc_method.clone(),
        auth_required: r.auth_required,
        rate_limit_per_second: r.rate_limit_per_second as i32,
        rate_limit_burst: r.rate_limit_burst as i32,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn model_to_domain(m: RouteModel) -> RouteDefinition {
    RouteDefinition {
        route_id: m.route_id,
        path: m.path,
        method: m.method,
        grpc_service: m.grpc_service,
        grpc_method: m.grpc_method,
        auth_required: m.auth_required,
        rate_limit_per_second: m.rate_limit_per_second as u32,
        rate_limit_burst: m.rate_limit_burst as u32,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

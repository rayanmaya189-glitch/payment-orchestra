//! API Gateway query handlers

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::queries::types::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_route(&self, method: &str, path: &str) -> Result<RouteDefinition, GatewayError>;
    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError>;
    async fn get_request(&self, request_id: Uuid) -> Result<ProcessedRequest, GatewayError>;
    async fn health_check(&self) -> GatewayHealth;
}

pub struct GatewayQueryHandler<R: GatewayRepository> {
    repo: R,
    startup_time: std::time::Instant,
}

impl<R: GatewayRepository> GatewayQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo, startup_time: std::time::Instant::now() }
    }
}

#[async_trait]
impl<R: GatewayRepository + Send + Sync> QueryHandler for GatewayQueryHandler<R> {
    async fn get_route(&self, method: &str, path: &str) -> Result<RouteDefinition, GatewayError> {
        self.repo.get_route(method, path).await?
            .ok_or_else(|| GatewayError::RouteNotFound(method.into(), path.into()))
    }

    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError> {
        self.repo.list_routes().await
    }

    async fn get_request(&self, request_id: Uuid) -> Result<ProcessedRequest, GatewayError> {
        self.repo.get_request_log(request_id).await?
            .ok_or_else(|| GatewayError::InternalError("Request not found".into()))
    }

    async fn health_check(&self) -> GatewayHealth {
        GatewayHealth {
            is_healthy: true,
            routes_count: default_routes().len(),
            uptime_hours: self.startup_time.elapsed().as_secs() / 3600,
            message: "API Gateway operational".into(),
        }
    }
}

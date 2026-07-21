use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::aggregates::{GatewayRequest, GatewayResponse, GatewayStats, RouteConfig};
use crate::domain::entities::{ForwardResult, RouteHealth, RouteRateLimitState};
use crate::domain::rules::{
    DefaultRouteMatcher, RequestForwarder, RouteMatcher, RouteRepository, StubForwarder,
};
use crate::domain::value_objects::{ForwardStatus, RateLimitConfig};
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};
use uuid::Uuid;

pub struct ApiGatewayServiceImpl {
    repo: Box<dyn RouteRepository>,
    db: DatabaseConnection,
    matcher: Arc<dyn RouteMatcher>,
    forwarder: Arc<dyn RequestForwarder>,
    rate_limit_states: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, RouteRateLimitState>>>,
    health_states: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<uuid::Uuid, RouteHealth>>>,
    stats: std::sync::Arc<tokio::sync::RwLock<GatewayStats>>,
}

impl ApiGatewayServiceImpl {
    pub fn new(repo: Box<dyn RouteRepository>, db: DatabaseConnection) -> Self {
        Self {
            repo,
            db,
            matcher: Arc::new(DefaultRouteMatcher::new()),
            forwarder: Arc::new(StubForwarder::new()),
            rate_limit_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            health_states: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            stats: Arc::new(tokio::sync::RwLock::new(GatewayStats::new())),
        }
    }

    fn make_rate_limit_key(route_id: Uuid, principal_id: &str) -> String {
        format!("{}:{}", route_id, principal_id)
    }
}

#[async_trait]
pub trait ApiGatewayService: Send + Sync {
    async fn resolve_route(&self, cmd: RouteRequest) -> Result<RouteConfig, PlatformError>;
    async fn list_routes(&self) -> Result<RouteListQueryResult, PlatformError>;
    async fn forward_request(&self, cmd: ForwardRequestCommand) -> Result<ForwardResultQueryResult, PlatformError>;
    async fn create_route(&self, cmd: CreateRouteCommand) -> Result<RouteConfig, PlatformError>;
    async fn update_route(&self, cmd: UpdateRouteCommand) -> Result<(), PlatformError>;
    async fn delete_route(&self, cmd: DeleteRouteCommand) -> Result<(), PlatformError>;
    async fn get_route_health(&self, cmd: GetRouteHealthCommand) -> Result<RouteHealthQueryResult, PlatformError>;
    async fn get_stats(&self) -> Result<GatewayStatsQueryResult, PlatformError>;
}

#[async_trait]
impl ApiGatewayService for ApiGatewayServiceImpl {
    async fn resolve_route(&self, cmd: RouteRequest) -> Result<RouteConfig, PlatformError> {
        let ctx = AbacContext {
            principal_id: Uuid::nil(),
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "route".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        let routes = self.repo.list_routes().await?;
        self.matcher
            .match_route(&cmd.path, &routes)
            .cloned()
            .ok_or_else(|| PlatformError::NotFound {
                resource: "route".into(),
                id: Uuid::nil(),
            })
    }

    async fn list_routes(&self) -> Result<RouteListQueryResult, PlatformError> {
        let routes = self.repo.list_routes().await?;
        Ok(RouteListQueryResult { routes })
    }

    async fn forward_request(&self, cmd: ForwardRequestCommand) -> Result<ForwardResultQueryResult, PlatformError> {
        let ctx = AbacContext {
            principal_id: Uuid::nil(),
            role: "api_client".to_string(),
            action: "read".to_string(),
            resource: "route".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        let routes = self.repo.list_routes().await?;
        let route = self
            .matcher
            .match_route(&cmd.path, &routes)
            .ok_or_else(|| PlatformError::NotFound {
                resource: "route".into(),
                id: Uuid::nil(),
            })?;

        // Check rate limit
        let principal_id = cmd.principal_id.as_deref().unwrap_or("anonymous");
        if let Some(ref rate_config) = route.rate_limit {
            let key = Self::make_rate_limit_key(Uuid::nil(), principal_id);
            let mut states = self.rate_limit_states.write().await;
            let state = states.entry(key.clone()).or_insert_with(|| {
                RouteRateLimitState::new(
                    Uuid::nil(),
                    route.path_prefix.clone(),
                    Uuid::nil(),
                    rate_config.clone(),
                )
            });
            state.reset_if_needed();

            if !state.is_within_limit() {
                let retry_after = state.seconds_until_reset() as u64;
                let mut stats = self.stats.write().await;
                stats.record_rate_limited();
                return Ok(ForwardResultQueryResult {
                    result: Some(
                        ForwardResult::new(
                            cmd.method.clone(),
                            cmd.path.clone(),
                            route.target_url.clone(),
                            route.target_service.clone(),
                        )
                        .with_status(ForwardStatus::RateLimited),
                    ),
                });
            }
            state.increment();
        }

        // Build request and forward
        let mut request = GatewayRequest::new(cmd.method.clone(), cmd.path.clone());
        request.headers = cmd.headers;
        request.body = cmd.body;
        request.principal_id = cmd.principal_id.clone();

        let start = std::time::Instant::now();
        let response = self.forwarder.forward(&request, &route.target_url).await?;
        let elapsed = start.elapsed().as_millis() as u64;

        let status = if response.is_success() {
            ForwardStatus::Success
        } else {
            ForwardStatus::UpstreamError
        };

        let mut result = ForwardResult::new(
            cmd.method,
            cmd.path,
            route.target_url.clone(),
            route.target_service.clone(),
        )
        .with_status(status)
        .with_response(response.status, elapsed);

        self.forwarder.log_forward(&result);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.record_request(elapsed, result.is_success());

        Ok(ForwardResultQueryResult {
            result: Some(result),
        })
    }

    async fn create_route(&self, cmd: CreateRouteCommand) -> Result<RouteConfig, PlatformError> {
        let ctx = AbacContext {
            principal_id: Uuid::nil(),
            role: "platform_admin".to_string(),
            action: "create".to_string(),
            resource: "route".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        let mut route = RouteConfig::new(
            cmd.path_prefix,
            cmd.target_service,
            cmd.target_url,
        )
        .with_auth_required(cmd.auth_required);

        if let (Some(max), Some(window)) = (cmd.rate_limit_max_requests, cmd.rate_limit_window_seconds) {
            route = route.with_rate_limit(max, window);
        }
        if let Some(methods) = cmd.methods {
            route = route.with_methods(methods);
        }
        if let Some(timeout) = cmd.timeout_ms {
            route = route.with_timeout(timeout);
        }

        let created = self.repo.create_route(&route).await?;
        Ok(created)
    }

    async fn update_route(&self, cmd: UpdateRouteCommand) -> Result<(), PlatformError> {
        let ctx = AbacContext {
            principal_id: Uuid::nil(),
            role: "platform_admin".to_string(),
            action: "update".to_string(),
            resource: "route".to_string(),
            resource_id: Some(cmd.route_id),
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        let existing = self.repo.find_route_by_id(cmd.route_id).await?.ok_or_else(|| {
            PlatformError::NotFound {
                resource: "route".into(),
                id: cmd.route_id,
            }
        })?;

        let mut updated = existing;
        if let Some(prefix) = cmd.path_prefix {
            updated.path_prefix = prefix;
        }
        if let Some(service) = cmd.target_service {
            updated.target_service = service;
        }
        if let Some(url) = cmd.target_url {
            updated.target_url = url;
        }
        if let Some(auth) = cmd.auth_required {
            updated.auth_required = auth;
        }
        updated.updated_at = chrono::Utc::now();

        self.repo.update_route(cmd.route_id, &updated).await
    }

    async fn delete_route(&self, cmd: DeleteRouteCommand) -> Result<(), PlatformError> {
        let ctx = AbacContext {
            principal_id: Uuid::nil(),
            role: "platform_admin".to_string(),
            action: "delete".to_string(),
            resource: "route".to_string(),
            resource_id: Some(cmd.route_id),
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        self.repo.delete_route(cmd.route_id).await
    }

    async fn get_route_health(&self, cmd: GetRouteHealthCommand) -> Result<RouteHealthQueryResult, PlatformError> {
        let states = self.health_states.read().await;
        let health = states.get(&cmd.route_id).cloned();
        Ok(RouteHealthQueryResult { health })
    }

    async fn get_stats(&self) -> Result<GatewayStatsQueryResult, PlatformError> {
        let stats = self.stats.read().await.clone();
        Ok(GatewayStatsQueryResult { stats })
    }
}

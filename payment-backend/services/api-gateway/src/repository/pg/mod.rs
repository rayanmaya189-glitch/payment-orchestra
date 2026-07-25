//! PostgreSQL-backed GatewayRepository using SeaORM CRUD.
//!
//! Route definitions are stored in the `route_definitions` table.
//! Rate limiting counters and request logs use in-memory storage since
//! these are performance-sensitive and typically use Redis in production.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::entities::route_definition::{
    Column as RouteColumn,
    Entity as RouteEntity, Model as RouteModel,
};

/// PostgreSQL-backed repository implementing GatewayRepository.
#[derive(Clone)]
pub struct PostgresGatewayRepository {
    pub db: sea_orm::DatabaseConnection,
    /// In-memory rate limit counters: key → (window_start_epoch, count)
    rate_limits: Arc<RwLock<HashMap<String, (i64, u32)>>>,
    /// In-memory API key → API key ID for validation
    api_keys: Arc<RwLock<HashMap<String, Uuid>>>,
    /// In-memory request log: request_id → ProcessedRequest
    request_log: Arc<RwLock<HashMap<Uuid, ProcessedRequest>>>,
}

impl PostgresGatewayRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self {
            db,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            request_log: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a known API key for validation (used for testing).
    pub fn add_api_key(&self, key: String, principal_id: Uuid) {
        let mut keys = self.api_keys.blocking_write();
        keys.insert(key, principal_id);
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

#[allow(dead_code)]
fn domain_to_model(route: &RouteDefinition) -> RouteModel {
    let (rate_limit_per_second, rate_limit_burst) = match &route.rate_limit_config {
        Some(config) => (config.max_requests as i32, config.window_seconds as i32),
        None => (0, 0),
    };

    RouteModel {
        route_id: route.route_id,
        path: route.url_pattern.clone(),
        method: route.http_method.clone(),
        grpc_service: route.grpc_service.clone(),
        grpc_method: route.grpc_method.clone(),
        auth_required: route.auth_required,
        rate_limit_per_second,
        rate_limit_burst,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn model_to_domain(m: RouteModel) -> RouteDefinition {
    let rate_limit_config = if m.rate_limit_per_second > 0 {
        Some(RateLimitConfig {
            max_requests: m.rate_limit_per_second as u32,
            window_seconds: m.rate_limit_burst as u32,
            per: RateLimitScope::ApiKey,
        })
    } else {
        None
    };

    let method = m.method.clone();
    let path = m.path.clone();

    RouteDefinition {
        route_id: m.route_id,
        http_method: m.method,
        url_pattern: m.path,
        grpc_service: m.grpc_service,
        grpc_method: m.grpc_method,
        path_params: vec![],
        rate_limit_config,
        auth_required: m.auth_required,
        description: format!("{} {}", method, path),
    }
}

// ─── GatewayRepository Trait Implementation ──────────────────────────────────

use crate::repository::GatewayRepository;

#[async_trait]
impl GatewayRepository for PostgresGatewayRepository {
    async fn get_route(&self, method: &str, path: &str) -> Result<Option<RouteDefinition>, GatewayError> {
        // Try exact match first, then wildcard patterns
        let result = RouteEntity::find()
            .filter(RouteColumn::Method.eq(method))
            .filter(RouteColumn::Path.eq(path))
            .one(&self.db)
            .await
            .map_err(|e| GatewayError::InternalError(format!("Database error: {e}")))?;

        if let Some(model) = result {
            return Ok(Some(model_to_domain(model)));
        }

        // Fallback: try wildcard pattern matching (path contains *)
        let all = RouteEntity::find()
            .filter(RouteColumn::Method.eq(method))
            .all(&self.db)
            .await
            .map_err(|e| GatewayError::InternalError(format!("Database error: {e}")))?;

        for model in all {
            let route = model_to_domain(model);
            if wildcard_match(&route.url_pattern, path) {
                return Ok(Some(route));
            }
        }

        Ok(None)
    }

    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError> {
        let results = RouteEntity::find()
            .order_by_asc(RouteColumn::Path)
            .all(&self.db)
            .await
            .map_err(|e| GatewayError::InternalError(format!("Database error: {e}")))?;

        Ok(results.into_iter().map(model_to_domain).collect())
    }

    async fn check_rate_limit(&self, key: &str, config: &RateLimitConfig) -> Result<bool, GatewayError> {
        let now = chrono::Utc::now().timestamp();
        let window_start = now - now % config.window_seconds as i64;

        let mut limits = self.rate_limits.write().await;
        let entry = limits.entry(key.to_string()).or_insert((window_start, 0));

        if entry.0 != window_start {
            // New window
            *entry = (window_start, 1);
            Ok(true)
        } else if entry.1 < config.max_requests {
            entry.1 += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn validate_api_key(&self, api_key: &str) -> Result<AuthResult, GatewayError> {
        let keys = self.api_keys.read().await;
        match keys.get(api_key) {
            Some(principal_id) => Ok(AuthResult {
                authenticated: true,
                actor_id: Some(*principal_id),
                actor_type: Some(ActorType::ApiKey),
                scopes: vec!["payments:read".into(), "payments:write".into()],
                error: None,
            }),
            None => Ok(AuthResult {
                authenticated: false,
                actor_id: None,
                actor_type: None,
                scopes: vec![],
                error: Some("Invalid API key".into()),
            }),
        }
    }

    async fn log_request(&self, request: &ProcessedRequest) -> Result<(), GatewayError> {
        let mut log = self.request_log.write().await;
        log.insert(request.request_id, request.clone());
        Ok(())
    }

    async fn get_request_log(&self, request_id: Uuid) -> Result<Option<ProcessedRequest>, GatewayError> {
        let log = self.request_log.read().await;
        Ok(log.get(&request_id).cloned())
    }
}

/// Simple wildcard pattern matching: `*` matches any sequence of characters.
fn wildcard_match(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        path.starts_with(prefix)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_match() {
        assert!(wildcard_match("/v1/payment-intents/*", "/v1/payment-intents/abc-123"));
        assert!(wildcard_match("/v1/payment-intents/*", "/v1/payment-intents/"));
        assert!(!wildcard_match("/v1/payment-intents/*", "/v1/invoices/abc"));
        assert!(wildcard_match("/v1/payment-intents", "/v1/payment-intents"));
        assert!(!wildcard_match("/v1/payment-intents", "/v1/payment-intents/extra"));
    }

    #[test]
    fn test_route_domain_entity_roundtrip() {
        let route = RouteDefinition {
            route_id: Uuid::now_v7(),
            http_method: "POST".into(),
            url_pattern: "/v1/payment-intents".into(),
            grpc_service: "orchestration.v1.OrchestrationService".into(),
            grpc_method: "CreatePaymentIntent".into(),
            path_params: vec![],
            rate_limit_config: Some(RateLimitConfig {
                max_requests: 1000,
                window_seconds: 60,
                per: RateLimitScope::ApiKey,
            }),
            auth_required: true,
            description: "Create payment intent".into(),
        };

        let model = domain_to_model(&route);
        let roundtrip = model_to_domain(model);

        assert_eq!(roundtrip.route_id, route.route_id);
        assert_eq!(roundtrip.http_method, "POST");
        assert_eq!(roundtrip.url_pattern, "/v1/payment-intents");
        assert_eq!(roundtrip.grpc_service, "orchestration.v1.OrchestrationService");
        assert_eq!(roundtrip.auth_required, true);
        assert!(roundtrip.rate_limit_config.is_some());
        assert_eq!(roundtrip.rate_limit_config.as_ref().unwrap().max_requests, 1000);
    }
}

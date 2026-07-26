//! In-memory API Gateway repository.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::GatewayRepository;

#[derive(Clone)]
pub struct InMemoryGatewayRepository {
    pub(super) routes: Arc<Vec<RouteDefinition>>,
    pub(super) rate_limit_counters: Arc<RwLock<HashMap<String, (u32, std::time::Instant)>>>,
    pub(super) api_keys: Arc<RwLock<HashMap<String, Uuid>>>,
    pub(super) request_log: Arc<RwLock<HashMap<Uuid, ProcessedRequest>>>,
}

impl Default for InMemoryGatewayRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGatewayRepository {
    /// Create a new in-memory gateway repository.
    ///
    /// Pre-seeds a well-known test API key for development and testing:
    /// `sk_test_12345` maps to a known operator UUID.
    /// In production, use `PostgresGatewayRepository` with real API keys.
    pub fn new() -> Self {
        let mut api_keys = HashMap::new();
        // Seed a well-known test API key for development/testing
        api_keys.insert(
            "sk_test_12345".to_string(),
            Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001),
        );
        Self {
            routes: Arc::new(default_routes()),
            rate_limit_counters: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(api_keys)),
            request_log: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl GatewayRepository for InMemoryGatewayRepository {
    async fn get_route(&self, method: &str, path: &str) -> Result<Option<RouteDefinition>, GatewayError> {
        Ok(self.routes.iter().find(|r| {
            if r.http_method != method { return false; }
            let route_segments: Vec<&str> = r.url_pattern.split('/').collect();
            let path_segments: Vec<&str> = path.split('/').collect();
            if route_segments.len() != path_segments.len() { return false; }
            route_segments.iter().zip(path_segments.iter()).all(|(r, p)| *r == *p || *r == "*")
        }).cloned())
    }

    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError> {
        Ok(self.routes.to_vec())
    }

    async fn check_rate_limit(&self, key: &str, config: &RateLimitConfig) -> Result<bool, GatewayError> {
        let mut counters = self.rate_limit_counters.write().await;
        let now = std::time::Instant::now();
        let entry = counters.entry(key.into()).or_insert((0, now));
        let elapsed = now.duration_since(entry.1);
        if elapsed.as_secs() >= config.window_seconds as u64 {
            *entry = (1, now);
            return Ok(true);
        }
        if entry.0 >= config.max_requests { return Ok(false); }
        entry.0 += 1;
        Ok(true)
    }

    async fn validate_api_key(&self, api_key: &str) -> Result<AuthResult, GatewayError> {
        let keys = self.api_keys.read().await;
        match keys.get(api_key) {
            Some(actor_id) => Ok(AuthResult {
                authenticated: true, actor_id: Some(*actor_id),
                actor_type: Some(ActorType::ApiKey), scopes: vec!["full_access".into()], error: None,
            }),
            None => Ok(AuthResult {
                authenticated: false, actor_id: None, actor_type: None,
                scopes: vec![], error: Some("Invalid API key".into()),
            }),
        }
    }

    async fn log_request(&self, request: &ProcessedRequest) -> Result<(), GatewayError> {
        self.request_log.write().await.insert(request.request_id, request.clone());
        Ok(())
    }

    async fn get_request_log(&self, request_id: Uuid) -> Result<Option<ProcessedRequest>, GatewayError> {
        Ok(self.request_log.read().await.get(&request_id).cloned())
    }
}

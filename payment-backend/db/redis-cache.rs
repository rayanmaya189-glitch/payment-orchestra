//! Redis caching for gateway health and routing policies.
//!
//! This module provides:
//! - Gateway health score caching
//! - Routing policy caching
//! - Connection status caching

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Gateway health status cached in Redis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayHealthCache {
    pub gateway_id: Uuid,
    pub is_healthy: bool,
    pub success_rate: f64,
    pub avg_latency_ms: u64,
    pub circuit_breaker_state: CircuitBreakerState,
    pub last_checked: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

/// Circuit breaker state for gateway health.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Routing policy cached in Redis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingPolicyCache {
    pub policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub rules: Vec<CachedRoutingRule>,
    pub failover_config: CachedFailoverConfig,
    pub rotation_strategy: String,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

/// Cached routing rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedRoutingRule {
    pub acquirer_link_id: Uuid,
    pub priority: i32,
    pub card_schemes: Option<Vec<String>>,
    pub currencies: Option<Vec<String>>,
}

/// Cached failover configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedFailoverConfig {
    pub max_hops: u8,
    pub latency_budget_ms: u32,
}

/// Redis cache manager for the payment orchestration platform.
pub struct RedisCacheManager {
    /// In-memory cache (will be backed by Redis in production)
    gateway_health: Arc<RwLock<HashMap<Uuid, GatewayHealthCache>>>,
    routing_policies: Arc<RwLock<HashMap<Uuid, RoutingPolicyCache>>>,
    operator_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    /// Default TTL for cache entries (seconds)
    default_ttl: u64,
}

impl RedisCacheManager {
    pub fn new() -> Self {
        Self {
            gateway_health: Arc::new(RwLock::new(HashMap::new())),
            routing_policies: Arc::new(RwLock::new(HashMap::new())),
            operator_policies: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: 300, // 5 minutes default
        }
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.default_ttl = ttl;
        self
    }

    // ─── Gateway Health Cache ────────────────────────────────────────────────

    /// Cache gateway health status.
    pub async fn cache_gateway_health(&self, health: GatewayHealthCache) {
        let mut cache = self.gateway_health.write().await;
        cache.insert(health.gateway_id, health);
    }

    /// Get cached gateway health.
    pub async fn get_gateway_health(&self, gateway_id: &Uuid) -> Option<GatewayHealthCache> {
        let cache = self.gateway_health.read().await;
        cache.get(gateway_id).cloned()
    }

    /// Get all cached gateway health statuses.
    pub async fn get_all_gateway_health(&self) -> HashMap<Uuid, GatewayHealthCache> {
        let cache = self.gateway_health.read().await;
        cache.clone()
    }

    /// Remove expired gateway health entries.
    pub async fn cleanup_expired_gateway_health(&self) {
        let now = chrono::Utc::now();
        let mut cache = self.gateway_health.write().await;
        cache.retain(|_, health| {
            let elapsed = now.signed_duration_since(health.last_checked);
            elapsed.num_seconds() < health.ttl_seconds as i64
        });
    }

    // ─── Routing Policy Cache ────────────────────────────────────────────────

    /// Cache routing policy.
    pub async fn cache_routing_policy(&self, policy: RoutingPolicyCache) {
        let mut policies = self.routing_policies.write().await;
        let mut operator_policies = self.operator_policies.write().await;
        
        operator_policies.insert(policy.operator_id, policy.policy_id);
        policies.insert(policy.policy_id, policy);
    }

    /// Get cached routing policy by ID.
    pub async fn get_routing_policy(&self, policy_id: &Uuid) -> Option<RoutingPolicyCache> {
        let policies = self.routing_policies.read().await;
        policies.get(policy_id).cloned()
    }

    /// Get cached routing policy for operator.
    pub async fn get_routing_policy_for_operator(&self, operator_id: &Uuid) -> Option<RoutingPolicyCache> {
        let operator_policies = self.operator_policies.read().await;
        let policies = self.routing_policies.read().await;
        
        if let Some(policy_id) = operator_policies.get(operator_id) {
            return policies.get(policy_id).cloned();
        }
        None
    }

    /// Invalidate routing policy cache.
    pub async fn invalidate_routing_policy(&self, policy_id: &Uuid) {
        let mut policies = self.routing_policies.write().await;
        if let Some(policy) = policies.remove(policy_id) {
            let mut operator_policies = self.operator_policies.write().await;
            operator_policies.remove(&policy.operator_id);
        }
    }

    // ─── Cache Statistics ────────────────────────────────────────────────────

    /// Get cache statistics.
    pub async fn get_stats(&self) -> CacheStats {
        let gateway_health = self.gateway_health.read().await;
        let routing_policies = self.routing_policies.read().await;
        
        CacheStats {
            gateway_health_entries: gateway_health.len(),
            routing_policy_entries: routing_policies.len(),
            memory_usage_bytes: 0, // Would calculate actual memory in production
        }
    }
}

impl Default for RedisCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub gateway_health_entries: usize,
    pub routing_policy_entries: usize,
    pub memory_usage_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_health_cache() {
        let cache = RedisCacheManager::new();
        let gateway_id = Uuid::now_v7();
        
        let health = GatewayHealthCache {
            gateway_id,
            is_healthy: true,
            success_rate: 0.95,
            avg_latency_ms: 150,
            circuit_breaker_state: CircuitBreakerState::Closed,
            last_checked: chrono::Utc::now(),
            ttl_seconds: 300,
        };
        
        cache.cache_gateway_health(health.clone()).await;
        let cached = cache.get_gateway_health(&gateway_id).await;
        
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.gateway_id, gateway_id);
        assert!(cached.is_healthy);
        assert!((cached.success_rate - 0.95).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_routing_policy_cache() {
        let cache = RedisCacheManager::new();
        let policy_id = Uuid::now_v7();
        let operator_id = Uuid::now_v7();
        
        let policy = RoutingPolicyCache {
            policy_id,
            operator_id,
            version: 1,
            rules: vec![],
            failover_config: CachedFailoverConfig {
                max_hops: 3,
                latency_budget_ms: 10000,
            },
            rotation_strategy: "Priority".into(),
            last_updated: chrono::Utc::now(),
            ttl_seconds: 300,
        };
        
        cache.cache_routing_policy(policy.clone()).await;
        
        // Get by policy ID
        let cached = cache.get_routing_policy(&policy_id).await;
        assert!(cached.is_some());
        
        // Get by operator ID
        let cached = cache.get_routing_policy_for_operator(&operator_id).await;
        assert!(cached.is_some());
        
        // Invalidate
        cache.invalidate_routing_policy(&policy_id).await;
        let cached = cache.get_routing_policy(&policy_id).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = RedisCacheManager::new();
        let stats = cache.get_stats().await;
        assert_eq!(stats.gateway_health_entries, 0);
        assert_eq!(stats.routing_policy_entries, 0);
    }
}

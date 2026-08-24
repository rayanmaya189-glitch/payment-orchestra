//! Redis-backed cache for the Payment Orchestra platform.
//!
//! Provides caching for:
//! - Gateway health scores and circuit breaker state
//! - Routing policies
//! - API key validation results
//! - Session data
//!
//! Uses Redis INCR + EXPIRE for rate limiting, HSET/HGET for structured data,
//! and sorted sets for sliding window statistics.

use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

// ─── Cache Manager ───────────────────────────────────────────────────────────

/// Redis-backed cache manager for the platform.
#[derive(Clone)]
pub struct CacheManager {
    redis: ConnectionManager,
    #[allow(dead_code)]
    default_ttl_secs: u64,
}

impl CacheManager {
    /// Create a new cache manager with a Redis connection.
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            default_ttl_secs: 300, // 5 minutes default
        }
    }

    /// Create a new cache manager with a custom default TTL.
    pub fn with_ttl(redis: ConnectionManager, ttl_secs: u64) -> Self {
        Self {
            redis,
            default_ttl_secs: ttl_secs,
        }
    }

    /// Get the underlying Redis connection manager.
    pub fn redis(&self) -> &ConnectionManager {
        &self.redis
    }

    // ─── Generic Cache Operations ─────────────────────────────────────────

    /// Set a key with a value and TTL.
    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> Result<(), CacheError> {
        let json = serde_json::to_string(value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        let _: () = redis::cmd("SETEX")
            .arg(key)
            .arg(ttl_secs)
            .arg(&json)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(())
    }

    /// Get a cached value by key.
    pub async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, CacheError> {
        let json: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        match json {
            Some(j) => {
                let value = serde_json::from_str(&j)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete a cached key.
    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let _: () = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(())
    }

    /// Check if a key exists.
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let count: u64 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(count > 0)
    }

    // ─── Gateway Health Cache ─────────────────────────────────────────────

    /// Cache gateway health status.
    pub async fn cache_gateway_health(
        &self,
        gateway_id: &str,
        health: &GatewayHealthCache,
    ) -> Result<(), CacheError> {
        let key = format!("gw:health:{}", gateway_id);
        self.set_json(&key, health, health.ttl_secs).await
    }

    /// Get cached gateway health.
    pub async fn get_gateway_health(
        &self,
        gateway_id: &str,
    ) -> Result<Option<GatewayHealthCache>, CacheError> {
        let key = format!("gw:health:{}", gateway_id);
        self.get_json(&key).await
    }

    /// Cache all gateway health statuses in a single pipeline.
    pub async fn cache_all_gateway_health(
        &self,
        entries: &[(String, GatewayHealthCache)],
    ) -> Result<(), CacheError> {
        let mut pipeline = redis::pipe();
        for (gateway_id, health) in entries {
            let key = format!("gw:health:{}", gateway_id);
            let json = serde_json::to_string(health)
                .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        pipeline
            .set_ex(&key, &json, health.ttl_secs)
            .ignore();
        }
        pipeline
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(())
    }

    // ─── Routing Policy Cache ─────────────────────────────────────────────

    /// Cache a routing policy.
    pub async fn cache_routing_policy(
        &self,
        policy_id: &str,
        policy: &RoutingPolicyCache,
    ) -> Result<(), CacheError> {
        let key = format!("route:policy:{}", policy_id);
        self.set_json(&key, policy, policy.ttl_secs).await
    }

    /// Get a cached routing policy.
    pub async fn get_routing_policy(
        &self,
        policy_id: &str,
    ) -> Result<Option<RoutingPolicyCache>, CacheError> {
        let key = format!("route:policy:{}", policy_id);
        self.get_json(&key).await
    }

    /// Cache the active routing policy for an operator.
    pub async fn cache_active_policy_for_operator(
        &self,
        operator_id: &str,
        policy_id: &str,
    ) -> Result<(), CacheError> {
        let key = format!("route:operator:{}:active", operator_id);
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(600i64) // 10 minutes
            .arg(policy_id)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(())
    }

    /// Get the active routing policy ID for an operator.
    pub async fn get_active_policy_for_operator(
        &self,
        operator_id: &str,
    ) -> Result<Option<String>, CacheError> {
        let key = format!("route:operator:{}:active", operator_id);
        let result: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(result)
    }

    /// Invalidate routing policy cache.
    pub async fn invalidate_routing_policy(
        &self,
        policy_id: &str,
    ) -> Result<(), CacheError> {
        let key = format!("route:policy:{}", policy_id);
        self.delete(&key).await
    }

    // ─── API Key Validation Cache ─────────────────────────────────────────

    /// Cache API key validation result.
    pub async fn cache_api_key_validation(
        &self,
        key_prefix: &str,
        principal_id: &str,
        scopes: &[String],
    ) -> Result<(), CacheError> {
        let cache_key = format!("apikey:valid:{}", key_prefix);
        let value = ApiKeyValidationCache {
            principal_id: principal_id.to_string(),
            scopes: scopes.to_vec(),
            validated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.set_json(&cache_key, &value, 300).await // 5 minutes
    }

    /// Get cached API key validation.
    pub async fn get_api_key_validation(
        &self,
        key_prefix: &str,
    ) -> Result<Option<ApiKeyValidationCache>, CacheError> {
        let cache_key = format!("apikey:valid:{}", key_prefix);
        self.get_json(&cache_key).await
    }

    /// Invalidate API key cache (e.g., on rotation).
    pub async fn invalidate_api_key(
        &self,
        key_prefix: &str,
    ) -> Result<(), CacheError> {
        let cache_key = format!("apikey:valid:{}", key_prefix);
        self.delete(&cache_key).await
    }

    // ─── Session Cache ────────────────────────────────────────────────────

    /// Cache a user session.
    pub async fn cache_session(
        &self,
        session_id: &str,
        session: &SessionCache,
    ) -> Result<(), CacheError> {
        let key = format!("session:{}", session_id);
        self.set_json(&key, session, session.ttl_secs).await
    }

    /// Get a cached session.
    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionCache>, CacheError> {
        let key = format!("session:{}", session_id);
        self.get_json(&key).await
    }

    /// Invalidate a session.
    pub async fn invalidate_session(
        &self,
        session_id: &str,
    ) -> Result<(), CacheError> {
        let key = format!("session:{}", session_id);
        self.delete(&key).await
    }

    // ─── Sliding Window Statistics ────────────────────────────────────────

    /// Record a success/failure in the sliding window for a gateway.
    pub async fn record_gateway_result(
        &self,
        gateway_id: &str,
        success: bool,
    ) -> Result<(), CacheError> {
        let key = format!("sr:window:{}", gateway_id);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let member = if success {
            format!("1:{}", now_ms)
        } else {
            format!("0:{}", now_ms)
        };

        let mut pipeline = redis::pipe();
        pipeline
            .zadd(&key, &member, now_ms)
            .ignore();
        pipeline
            .expire(&key, 3600i64) // 1 hour window
            .ignore();
        pipeline
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        // Trim to keep only last 100 entries
        let count: u64 = redis::cmd("ZCARD")
            .arg(&key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        if count > 100 {
            let _: () = redis::cmd("ZREMRANGEBYRANK")
                .arg(&key)
                .arg(0)
                .arg(-101)
                .query_async(&mut self.redis.clone())
                .await
                .map_err(|e| CacheError::RedisError(e.to_string()))?;
        }

        Ok(())
    }

    /// Get the success rate for a gateway from the sliding window.
    pub async fn get_gateway_success_rate(
        &self,
        gateway_id: &str,
    ) -> Result<f64, CacheError> {
        let key = format!("sr:window:{}", gateway_id);
        let members: Vec<String> = redis::cmd("ZRANGE")
            .arg(&key)
            .arg(0)
            .arg(-1)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        if members.is_empty() {
            return Ok(1.0); // Default 100%
        }

        let successes = members.iter().filter(|m| m.starts_with("1:")).count();
        let total = members.len();

        Ok(successes as f64 / total as f64)
    }

    // ─── Health Check ─────────────────────────────────────────────────────

    /// Check if Redis is reachable.
    pub async fn ping(&self) -> Result<(), CacheError> {
        let _: String = redis::cmd("PING")
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;
        Ok(())
    }
}

// ─── Cache Types ─────────────────────────────────────────────────────────────

/// Gateway health status cached in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayHealthCache {
    pub gateway_id: String,
    pub is_healthy: bool,
    pub success_rate: f64,
    pub avg_latency_ms: u64,
    pub circuit_breaker_state: String, // "closed", "open", "half_open"
    pub last_checked: String,          // ISO 8601
    pub ttl_secs: u64,
}

/// Routing policy cached in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicyCache {
    pub policy_id: String,
    pub operator_id: String,
    pub version: i32,
    pub rules: Vec<CachedRoutingRule>,
    pub failover_max_hops: u8,
    pub failover_latency_budget_ms: u32,
    pub rotation_strategy: String,
    pub last_updated: String,
    pub ttl_secs: u64,
}

/// Cached routing rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRoutingRule {
    pub acquirer_link_id: String,
    pub priority: i32,
    pub card_schemes: Option<Vec<String>>,
    pub currencies: Option<Vec<String>>,
}

/// API key validation cached in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyValidationCache {
    pub principal_id: String,
    pub scopes: Vec<String>,
    pub validated_at: String,
}

/// Session cached in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCache {
    pub session_id: String,
    pub principal_id: String,
    pub operator_id: Option<String>,
    pub roles: Vec<String>,
    pub created_at: String,
    pub expires_at: String,
    pub ttl_secs: u64,
}

// ─── Error Types ─────────────────────────────────────────────────────────────

/// Cache operation errors.
#[derive(Debug, Clone)]
pub enum CacheError {
    RedisError(String),
    SerializationError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::RedisError(e) => write!(f, "Redis error: {}", e),
            CacheError::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<CacheError> for platform_error::PlatformError {
    fn from(e: CacheError) -> Self {
        platform_error::PlatformError::Internal(e.to_string())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_health_cache_serialization() {
        let health = GatewayHealthCache {
            gateway_id: "gw-123".into(),
            is_healthy: true,
            success_rate: 0.95,
            avg_latency_ms: 150,
            circuit_breaker_state: "closed".into(),
            last_checked: chrono::Utc::now().to_rfc3339(),
            ttl_secs: 30,
        };

        let json = serde_json::to_string(&health).unwrap();
        let deserialized: GatewayHealthCache = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.gateway_id, "gw-123");
        assert!(deserialized.is_healthy);
        assert!((deserialized.success_rate - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_routing_policy_cache_serialization() {
        let policy = RoutingPolicyCache {
            policy_id: "pol-456".into(),
            operator_id: "op-789".into(),
            version: 1,
            rules: vec![CachedRoutingRule {
                acquirer_link_id: "link-1".into(),
                priority: 1,
                card_schemes: Some(vec!["visa".into()]),
                currencies: None,
            }],
            failover_max_hops: 3,
            failover_latency_budget_ms: 10000,
            rotation_strategy: "priority".into(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            ttl_secs: 600,
        };

        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RoutingPolicyCache = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.policy_id, "pol-456");
        assert_eq!(deserialized.rules.len(), 1);
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::RedisError("connection refused".into());
        assert!(err.to_string().contains("connection refused"));

        let err = CacheError::SerializationError("invalid JSON".into());
        assert!(err.to_string().contains("invalid JSON"));
    }
}

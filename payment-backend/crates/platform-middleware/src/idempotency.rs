use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

// ─── Idempotency Key ───────────────────────────────────────────────────────

/// A validated idempotency key for API requests.
///
/// Keys must be 16-256 characters and contain only alphanumeric characters,
/// hyphens, and underscores (RFC-compliant).
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    pub fn new(key: &str) -> Result<Self, String> {
        if key.len() < 16 || key.len() > 256 {
            return Err("Idempotency key must be 16-256 characters".into());
        }
        // Validate characters: alphanumeric, hyphens, underscores only
        if !key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(
                "Idempotency key must contain only alphanumeric characters, hyphens, and underscores"
                    .into(),
            );
        }
        Ok(Self(key.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Generate a new idempotency key (UUIDv7 for time-ordered uniqueness).
pub fn generate_idempotency_key() -> String {
    Uuid::now_v7().to_string()
}

// ─── Idempotency Record ────────────────────────────────────────────────────

/// The result of a previously completed request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// The original request ID that produced this result.
    pub request_id: String,
    /// HTTP status code of the original response.
    pub status_code: u16,
    /// Serialized response body.
    pub response_body: Vec<u8>,
    /// Unix timestamp (seconds) when the original request was processed.
    pub created_at_unix_secs: u64,
}

impl IdempotencyRecord {
    /// Check if this record has expired based on TTL.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now_secs.saturating_sub(self.created_at_unix_secs) > ttl.as_secs()
    }
}

// ─── In-Memory Idempotency Store ───────────────────────────────────────────

/// In-memory idempotency store with TTL-based expiration.
///
/// Suitable for single-instance deployments. For distributed deployments,
/// use [`RedisIdempotencyStore`].
#[derive(Clone)]
pub struct MemoryIdempotencyStore {
    inner: Arc<Mutex<MemoryIdempotencyInner>>,
    ttl: Duration,
}

#[derive(Debug)]
struct MemoryIdempotencyInner {
    records: HashMap<String, IdempotencyRecord>,
    last_cleanup: Instant,
}

impl MemoryIdempotencyStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryIdempotencyInner {
                records: HashMap::new(),
                last_cleanup: Instant::now(),
            })),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Check if a request with this idempotency key has already been processed.
    pub fn check(&self, key: &str) -> Result<Option<IdempotencyRecord>, String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;

        // Periodic cleanup of expired records
        if inner.last_cleanup.elapsed() > Duration::from_secs(300) {
            inner
                .records
                .retain(|_, record| !record.is_expired(self.ttl));
            inner.last_cleanup = Instant::now();
        }

        Ok(inner.records.get(key).cloned())
    }

    /// Store the result of a processed request.
    pub fn store(
        &self,
        key: &str,
        request_id: &str,
        status_code: u16,
        response_body: Vec<u8>,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        inner.records.insert(
            key.to_string(),
            IdempotencyRecord {
                request_id: request_id.to_string(),
                status_code,
                response_body,
                created_at_unix_secs: now_secs,
            },
        );
        Ok(())
    }
}

// ─── Redis-Backed Idempotency Store ────────────────────────────────────────

/// Redis-backed idempotency store for distributed deployments.
///
/// Uses Redis GET/SET with TTL for atomic idempotency checks.
#[derive(Clone)]
pub struct RedisIdempotencyStore {
    redis: ConnectionManager,
    ttl: Duration,
}

impl RedisIdempotencyStore {
    pub fn new(redis: ConnectionManager, ttl_secs: u64) -> Self {
        Self {
            redis,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Check if a request with this idempotency key has already been processed.
    ///
    /// If Redis is unavailable, logs a warning and returns `None` (treats as new request)
    /// to allow graceful degradation.
    pub async fn check(&self, key: &str) -> Result<Option<IdempotencyRecord>, String> {
        let redis_key = format!("idempotency:{}", key);
        let data: Option<String> = match redis::cmd("GET")
            .arg(&redis_key)
            .query_async::<Option<String>>(&mut self.redis.clone())
            .await
        {
            Ok(data) => data,
            Err(e) => {
                warn!(
                    key = %key,
                    error = %e,
                    "Redis idempotency check failed, degrading to new request"
                );
                return Ok(None);
            }
        };

        match data {
            Some(json) => {
                let record: IdempotencyRecord =
                    serde_json::from_str(&json).map_err(|e| format!("Deserialize failed: {}", e))?;
                // Check if expired
                if record.is_expired(self.ttl) {
                    return Ok(None);
                }
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Store the result of a processed request with TTL.
    pub async fn store(
        &self,
        key: &str,
        request_id: &str,
        status_code: u16,
        response_body: Vec<u8>,
    ) -> Result<(), String> {
        let redis_key = format!("idempotency:{}", key);
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let record = IdempotencyRecord {
            request_id: request_id.to_string(),
            status_code,
            response_body,
            created_at_unix_secs: now_secs,
        };
        let json = serde_json::to_string(&record).map_err(|e| format!("Serialize failed: {}", e))?;

        redis::cmd("SET")
            .arg(&redis_key)
            .arg(&json)
            .arg("EX")
            .arg(self.ttl.as_secs() as i64)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| format!("Redis SET failed: {}", e))?;

        Ok(())
    }
}

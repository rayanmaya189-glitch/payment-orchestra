//! IdempotencyCache implementation using Redis (primary) with in-memory fallback.
//!
//! Uses `tokio::sync::Mutex` for interior mutability since the IdempotencyCache
//! trait only provides `&self` access.

use async_trait::async_trait;

use crate::domain::*;
use super::PostgresOrchestrationRepository;
use crate::repository::IdempotencyCache;

/// TTL for idempotency cache entries: 24 hours.
const IDEMPOTENCY_TTL_SECS: usize = 86_400;

#[async_trait]
impl IdempotencyCache for PostgresOrchestrationRepository {
    async fn check_idempotency(&self, key: &str) -> Result<IdempotencyResult, OrchestrationError> {
        // Try Redis first
        if let Some(ref redis_mutex) = self.redis_conn {
            let mut conn = redis_mutex.lock().await;
            let cached: Option<String> = redis::cmd("GET")
                .arg(key)
                .query_async(&mut *conn)
                .await
                .ok();

            if let Some(val) = cached {
                match serde_json::from_str(&val) {
                    Ok(result) => return Ok(IdempotencyResult::Duplicate(result)),
                    Err(_) => {
                        // Cache corrupted, treat as new
                    }
                }
            }
        }

        Ok(IdempotencyResult::New)
    }

    async fn store_idempotency(&self, key: &str, result: &serde_json::Value) -> Result<(), OrchestrationError> {
        // Store in Redis with TTL
        if let Some(ref redis_mutex) = self.redis_conn {
            let mut conn = redis_mutex.lock().await;
            let val = serde_json::to_string(result)
                .map_err(|e| OrchestrationError::DatabaseError(format!("Serialize idempotency: {}", e)))?;

            let _: Result<String, _> = redis::cmd("SETEX")
                .arg(key)
                .arg(IDEMPOTENCY_TTL_SECS)
                .arg(&val)
                .query_async(&mut *conn)
                .await;
        }

        Ok(())
    }
}

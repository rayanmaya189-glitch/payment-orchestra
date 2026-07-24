//! Rate limiting middleware — Redis-backed sliding window algorithm.
//!
//! Uses Redis INCR + EXPIRE for a fixed-window rate limit per key.
//! Windows are defined by a key (e.g., "ratelimit:{api_key_id}:{endpoint}"),
//! a max request count per window, and the window duration in seconds.

use platform_error::PlatformError;
use redis::aio::ConnectionManager;

/// Rate limiter backed by Redis using a sliding window approach.
///
/// Algorithm:
/// 1. INCR `ratelimit:{key}:{window_timestamp}`
/// 2. If the key was just created (return value == 1), set EXPIRE to `window_secs + 1`
/// 3. Compare the current count against the `limit`
/// 4. If count > limit, return RateLimited error
pub struct RateLimiter {
    redis: ConnectionManager,
}

impl RateLimiter {
    /// Create a new rate limiter with a Redis connection manager.
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Check if a request should be rate limited.
    ///
    /// # Arguments
    /// * `key` - Unique key for the rate limit bucket (e.g., `"ratelimit:{api_key_id}:{endpoint}"`)
    /// * `limit` - Maximum number of requests allowed within the window
    /// * `window_secs` - Time window in seconds
    ///
    /// # Returns
    /// * `Ok(true)` — request is allowed
    /// * `Ok(false)` — request is rate limited (but not an error, caller should handle)
    /// * `Err(PlatformError::RateLimited { retry_after_ms })` — rate limited with retry info
    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<bool, PlatformError> {
        let window_key = format!("ratelimit:{}", key);

        // Use INCR to atomically increment the counter
        let count: u32 = redis::cmd("INCR")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis INCR failed: {}", e)))?;

        // If this is the first request in the window, set expiration
        if count == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(&window_key)
                .arg(window_secs as i64)
                .query_async(&mut self.redis.clone())
                .await
                .map_err(|e| PlatformError::Internal(format!("Redis EXPIRE failed: {}", e)))?;
        }

        if count > limit {
            // Get remaining TTL for retry_after calculation
            let ttl: i64 = redis::cmd("TTL")
                .arg(&window_key)
                .query_async(&mut self.redis.clone())
                .await
                .unwrap_or(window_secs as i64);

            let retry_after_ms = (ttl.max(0) as u64) * 1000;
            return Err(PlatformError::RateLimited { retry_after_ms });
        }

        Ok(true)
    }

    /// Get the current count for a rate limit key (without incrementing).
    pub async fn current_count(&self, key: &str) -> Result<u32, PlatformError> {
        let window_key = format!("ratelimit:{}", key);
        let count: Option<u32> = redis::cmd("GET")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis GET failed: {}", e)))?;

        Ok(count.unwrap_or(0))
    }

    /// Reset a rate limit counter for a key.
    pub async fn reset(&self, key: &str) -> Result<(), PlatformError> {
        let window_key = format!("ratelimit:{}", key);
        let _: () = redis::cmd("DEL")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis DEL failed: {}", e)))?;
        Ok(())
    }

    /// Create a rate limit key from components.
    pub fn build_key(prefix: &str, identifier: &str, endpoint: &str) -> String {
        format!("{}:{}:{}", prefix, identifier, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_key() {
        let key = RateLimiter::build_key("ratelimit", "api_key_123", "payment_intents/create");
        assert_eq!(key, "ratelimit:api_key_123:payment_intents/create");
    }

    #[test]
    fn test_build_key_with_prefix_only() {
        let key = RateLimiter::build_key("ip", "192.168.1.1", "any");
        assert_eq!(key, "ip:192.168.1.1:any");
    }
}

use platform_config::RedisConfig;
use platform_db::connect_redis;
use redis::aio::ConnectionManager;

pub async fn connect(config: &RedisConfig) -> ConnectionManager {
    connect_redis(config).await
}

/// Redis-backed session store for refresh tokens and MFA state.
pub struct RedisSessionStore {
    redis: ConnectionManager,
}

impl RedisSessionStore {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    pub async fn store_refresh_token(
        &self,
        token_id: &str,
        principal_id: &str,
        ttl_secs: u64,
    ) -> Result<(), platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        redis::cmd("SET")
            .arg(format!("refresh_token:{}", token_id))
            .arg(principal_id)
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<()>(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis SET failed: {e}")))?;
        Ok(())
    }

    pub async fn get_refresh_token(
        &self,
        token_id: &str,
    ) -> Result<Option<String>, platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        let result: Option<String> = redis::cmd("GET")
            .arg(format!("refresh_token:{}", token_id))
            .query_async(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis GET failed: {e}")))?;
        Ok(result)
    }

    pub async fn delete_refresh_token(
        &self,
        token_id: &str,
    ) -> Result<(), platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        redis::cmd("DEL")
            .arg(format!("refresh_token:{}", token_id))
            .query_async::<()>(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis DEL failed: {e}")))?;
        Ok(())
    }

    pub async fn invalidate_all_principal_tokens(
        &self,
        principal_id: &str,
    ) -> Result<(), platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        let pattern = format!("refresh_token:*");
        let mut cursor = 0u64;
        loop {
            let result: (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut redis)
                .await
                .map_err(|e| platform_error::PlatformError::Internal(format!("Redis SCAN failed: {e}")))?;

            cursor = result.0;
            for key in &result.1 {
                if let Ok(Some(stored_principal)) = self.get_refresh_token(key).await {
                    if stored_principal == principal_id {
                        let _ = redis::cmd("DEL")
                            .arg(key)
                            .query_async::<()>(&mut redis)
                            .await;
                    }
                }
            }

            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Store an MFA challenge token with a TTL (SRS AUTH-001).
    pub async fn store_mfa_challenge(
        &self,
        challenge_token: &str,
        principal_id: &str,
        ttl_secs: u64,
    ) -> Result<(), platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        redis::cmd("SET")
            .arg(format!("mfa_challenge:{}", challenge_token))
            .arg(principal_id)
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<()>(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis SET failed: {e}")))?;
        Ok(())
    }

    /// Verify and consume an MFA challenge token (one-time use).
    pub async fn verify_mfa_challenge(
        &self,
        challenge_token: &str,
    ) -> Result<Option<String>, platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        let key = format!("mfa_challenge:{}", challenge_token);
        let result: Option<String> = redis::cmd("GETDEL")
            .arg(&key)
            .query_async(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis GETDEL failed: {e}")))?;
        Ok(result)
    }
}

/// Idempotency store for login attempts (per IP).
pub struct LoginAttemptStore {
    redis: ConnectionManager,
}

impl LoginAttemptStore {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    pub async fn increment_login_attempts(
        &self,
        ip: &str,
        window_secs: u64,
    ) -> Result<u32, platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        let key = format!("login_attempts:{}", ip);
        let count: u32 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis INCR failed: {e}")))?;

        if count == 1 {
            redis::cmd("EXPIRE")
                .arg(&key)
                .arg(window_secs)
                .query_async::<()>(&mut redis)
                .await
                .map_err(|e| platform_error::PlatformError::Internal(format!("Redis EXPIRE failed: {e}")))?;
        }

        Ok(count)
    }

    pub async fn get_login_attempts(
        &self,
        ip: &str,
    ) -> Result<u32, platform_error::PlatformError> {
        let mut redis = self.redis.clone();
        let count: u32 = redis::cmd("GET")
            .arg(format!("login_attempts:{}", ip))
            .query_async(&mut redis)
            .await
            .map_err(|e| platform_error::PlatformError::Internal(format!("Redis GET failed: {e}")))?;
        Ok(count)
    }
}

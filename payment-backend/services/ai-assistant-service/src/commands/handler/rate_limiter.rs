use async_trait::async_trait;
use uuid::Uuid;

use super::RateLimiter;
use crate::domain::*;

pub struct NoopRateLimiter;

#[async_trait]
impl RateLimiter for NoopRateLimiter {
    async fn check_rate_limit(&self, _operator_id: Uuid) -> Result<(), AiError> {
        Ok(())
    }
}

pub struct TokenBucketRateLimiter {
    max_per_minute: u32,
    state: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, (u32, std::time::Instant)>>>,
}

impl TokenBucketRateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            max_per_minute,
            state: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucketRateLimiter {
    async fn check_rate_limit(&self, operator_id: Uuid) -> Result<(), AiError> {
        let mut state = self.state.write().await;
        let now = std::time::Instant::now();

        let entry = state.entry(operator_id).or_insert((0, now));
        let elapsed = now.duration_since(entry.1);

        if elapsed.as_secs() >= 60 {
            *entry = (1, now);
            return Ok(());
        }

        if entry.0 >= self.max_per_minute {
            return Err(AiError::RateLimitExceeded("general".into()));
        }

        entry.0 += 1;
        Ok(())
    }
}

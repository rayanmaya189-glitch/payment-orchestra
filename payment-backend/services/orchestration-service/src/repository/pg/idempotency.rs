use async_trait::async_trait;

use super::PostgresOrchestrationRepository;
use crate::repository::IdempotencyCache;
use crate::domain::*;

#[async_trait]
impl IdempotencyCache for PostgresOrchestrationRepository {
    async fn check_idempotency(
        &self,
        key: &str,
    ) -> Result<IdempotencyResult, OrchestrationError> {
        let cache = self.idempotency_cache.read().await;
        match cache.get(key) {
            Some(result) => Ok(IdempotencyResult::Duplicate(result.clone())),
            None => Ok(IdempotencyResult::New),
        }
    }

    async fn store_idempotency(
        &self,
        key: &str,
        result: &serde_json::Value,
    ) -> Result<(), OrchestrationError> {
        let mut cache = self.idempotency_cache.write().await;
        cache.insert(key.to_string(), result.clone());
        Ok(())
    }
}

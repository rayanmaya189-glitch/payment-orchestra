//! IdempotencyCache implementation for InMemoryOrchestrationRepository.

use async_trait::async_trait;

use crate::domain::*;
use super::in_memory::InMemoryOrchestrationRepository;
use super::traits::IdempotencyCache;

#[async_trait]
impl IdempotencyCache for InMemoryOrchestrationRepository {
    async fn check_idempotency(&self, key: &str) -> Result<IdempotencyResult, OrchestrationError> {
        let cache = self.idempotency_cache.read().await;
        match cache.get(key) {
            Some(result) => Ok(IdempotencyResult::Duplicate(result.clone())),
            None => Ok(IdempotencyResult::New),
        }
    }

    async fn store_idempotency(&self, key: &str, result: &serde_json::Value) -> Result<(), OrchestrationError> {
        let mut cache = self.idempotency_cache.write().await;
        cache.insert(key.to_string(), result.clone());
        Ok(())
    }
}

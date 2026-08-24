//! Saga Coordinator repository trait — BC-17

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait SagaRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<SagaInstance>, SagaError>;
    async fn save(&self, saga: &SagaInstance) -> Result<(), SagaError>;
    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError>;
    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError>;
}

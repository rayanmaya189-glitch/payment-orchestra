//! Saga Coordinator query handlers — BC-17

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_saga(&self, id: Uuid) -> Result<SagaInstance, SagaError>;
    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError>;
    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError>;
}

pub struct SagaQueryHandler<R: SagaRepository> {
    repo: R,
}

impl<R: SagaRepository> SagaQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SagaRepository + Send + Sync> QueryHandler for SagaQueryHandler<R> {
    async fn get_saga(&self, id: Uuid) -> Result<SagaInstance, SagaError> {
        self.repo.load(id).await?.ok_or(SagaError::NotFound(id))
    }

    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError> {
        self.repo.find_by_aggregate(aggregate_id).await
    }

    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError> {
        self.repo.find_stuck(timeout_seconds).await
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_saga(&self, id: Uuid) -> Result<SagaInstance, SagaError> {
        (**self).get_saga(id).await
    }
    async fn find_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<SagaInstance>, SagaError> {
        (**self).find_by_aggregate(aggregate_id).await
    }
    async fn find_stuck(&self, timeout_seconds: i64) -> Result<Vec<SagaInstance>, SagaError> {
        (**self).find_stuck(timeout_seconds).await
    }
}

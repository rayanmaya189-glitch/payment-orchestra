//! Read-model queries for operator-service.

use uuid::Uuid;
use crate::domain::{Operator, OperatorError};
use crate::repository::OperatorRepository;

#[async_trait::async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_operator(&self, id: Uuid) -> Result<Option<Operator>, OperatorError>;
    async fn list_operators(&self, status_filter: Option<&str>) -> Result<Vec<Operator>, OperatorError>;
}

pub struct OperatorQueries<R: OperatorRepository> {
    repository: R,
}

impl<R: OperatorRepository> OperatorQueries<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R: OperatorRepository + Send + Sync> QueryHandler for OperatorQueries<R> {
    async fn get_operator(&self, id: Uuid) -> Result<Option<Operator>, OperatorError> {
        self.repository.load(id).await
    }

    async fn list_operators(&self, status_filter: Option<&str>) -> Result<Vec<Operator>, OperatorError> {
        self.repository.list_by_status(status_filter).await
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait::async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_operator(&self, id: Uuid) -> Result<Option<Operator>, OperatorError> {
        (**self).get_operator(id).await
    }

    async fn list_operators(&self, status_filter: Option<&str>) -> Result<Vec<Operator>, OperatorError> {
        (**self).list_operators(status_filter).await
    }
}

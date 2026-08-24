//! Query handler trait and implementation for dispute-service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_chargeback(&self, id: Uuid) -> Result<ChargebackCase, DisputeError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
}

pub struct DisputeQueryHandler<R: DisputeRepository> {
    repo: R,
}

impl<R: DisputeRepository> DisputeQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: DisputeRepository + Send + Sync> QueryHandler for DisputeQueryHandler<R> {
    async fn get_chargeback(&self, id: Uuid) -> Result<ChargebackCase, DisputeError> {
        self.repo.load(id).await?.ok_or(DisputeError::NotFound(id))
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.repo.find_by_payment_intent(payment_intent_id).await
    }

    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.repo.find_open_cases(operator_id).await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.repo.find_by_operator(operator_id).await
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_chargeback(&self, id: Uuid) -> Result<ChargebackCase, DisputeError> {
        (**self).get_chargeback(id).await
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        (**self).find_by_payment_intent(payment_intent_id).await
    }

    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        (**self).find_open_cases(operator_id).await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError> {
        (**self).find_by_operator(operator_id).await
    }
}

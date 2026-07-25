//! Query handler trait and implementation for subscription-service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, SubscriptionError>;
    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError>;
}

pub struct SubscriptionQueryHandler<R: SubscriptionRepository> {
    repo: R,
}

impl<R: SubscriptionRepository> SubscriptionQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SubscriptionRepository + Send + Sync> QueryHandler for SubscriptionQueryHandler<R> {
    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, SubscriptionError> {
        self.repo.load(id).await?.ok_or(SubscriptionError::NotFound(id))
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        self.repo.find_by_customer(customer_id).await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        self.repo.find_by_operator(operator_id).await
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        self.repo.find_active_for_renewal().await
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_subscription(&self, id: Uuid) -> Result<Subscription, SubscriptionError> {
        (**self).get_subscription(id).await
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        (**self).find_by_customer(customer_id).await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        (**self).find_by_operator(operator_id).await
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        (**self).find_active_for_renewal().await
    }
}

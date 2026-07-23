//! Subscription Billing repository trait — BC-08

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError>;
    async fn save(&self, subscription: &Subscription) -> Result<(), SubscriptionError>;
    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
}

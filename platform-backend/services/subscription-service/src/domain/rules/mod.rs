use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Subscription;
use platform_error::PlatformError;

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Subscription>, PlatformError>;
    async fn save(&self, subscription: &Subscription) -> Result<(), PlatformError>;
    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
    async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError>;
}

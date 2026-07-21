use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Subscription;
use platform_error::PlatformError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionSortField {
    CreatedAt,
    CurrentPeriodEnd,
    Amount,
}

#[derive(Debug, Clone)]
pub struct SubscriptionFilter {
    pub status: Option<String>,
    pub customer_id: Option<Uuid>,
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub limit: i64,
    pub offset: i64,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: 20,
            offset: 0,
        }
    }
}

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Subscription>, PlatformError>;
    async fn save(&self, subscription: &Subscription) -> Result<(), PlatformError>;
    async fn list_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        filter: &SubscriptionFilter,
        pagination: &PaginationParams,
    ) -> Result<Vec<Subscription>, PlatformError>;
    async fn find_due_subscriptions(&self) -> Result<Vec<Subscription>, PlatformError>;
    async fn find_canceled_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Subscription>, PlatformError>;
}

use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::PaymentLink;
use platform_error::PlatformError;

#[derive(Debug, Clone)]
pub struct PaymentLinkFilter {
    pub status: Option<String>,
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
pub trait PaymentLinkRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PaymentLink>, PlatformError>;
    async fn find_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PlatformError>;
    async fn save(&self, link: &PaymentLink) -> Result<(), PlatformError>;
    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        filter: &PaymentLinkFilter,
        pagination: &PaginationParams,
    ) -> Result<Vec<PaymentLink>, PlatformError>;
    async fn find_expired_links(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PaymentLink>, PlatformError>;
}

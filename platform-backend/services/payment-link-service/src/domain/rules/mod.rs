use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::PaymentLink;
use platform_error::PlatformError;

#[async_trait]
pub trait PaymentLinkRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PaymentLink>, PlatformError>;
    async fn find_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PlatformError>;
    async fn save(&self, link: &PaymentLink) -> Result<(), PlatformError>;
}

//! Payment Link repository trait — BC-07

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait PaymentLinkRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentLink>, PaymentLinkError>;
    async fn load_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PaymentLinkError>;
    async fn save(&self, link: &PaymentLink) -> Result<(), PaymentLinkError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError>;
    async fn find_expired(&self) -> Result<Vec<PaymentLink>, PaymentLinkError>;
}

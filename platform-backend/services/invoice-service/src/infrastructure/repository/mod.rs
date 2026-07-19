use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::Invoice;
use platform_error::PlatformError;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Invoice>, PlatformError>;
    async fn save(&self, invoice: &Invoice) -> Result<(), PlatformError>;
    async fn find_by_order_reference(&self, operator_id: Uuid, order_ref: &str) -> Result<Option<Invoice>, PlatformError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<Invoice>, PlatformError>;
}

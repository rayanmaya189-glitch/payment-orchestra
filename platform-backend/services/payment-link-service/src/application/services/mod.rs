use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::PaymentLink;
use platform_error::PlatformError;

#[async_trait]
pub trait PaymentLinkService: Send + Sync {
    async fn create_link(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLinkResponse, PlatformError>;
    async fn get_link(&self, link_id: Uuid) -> Result<PaymentLinkResponse, PlatformError>;
    async fn deactivate_link(&self, link_id: Uuid) -> Result<(), PlatformError>;
}

pub struct PaymentLinkServiceImpl { db: sea_orm::DatabaseConnection }
impl PaymentLinkServiceImpl { pub fn new(db: sea_orm::DatabaseConnection) -> Self { Self { db } } }

pub struct CreatePaymentLinkCommand { pub operator_id: Uuid, pub amount: shared_types::Money, pub description: String, pub merchant_name: String, pub expires_at: Option<chrono::DateTime<chrono::Utc>>, pub max_uses: Option<i32> }

#[derive(Debug, Clone)]
pub struct PaymentLinkResponse { pub link_id: Uuid, pub status: String, pub amount: i64, pub currency: String, pub description: String, pub merchant_name: String, pub current_uses: i32, pub created_at: String }

#[async_trait]
impl PaymentLinkService for PaymentLinkServiceImpl {
    async fn create_link(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLinkResponse, PlatformError> {
        let mut link = PaymentLink::new(cmd.operator_id, cmd.amount.clone(), cmd.description, cmd.merchant_name);
        link.expires_at = cmd.expires_at;
        link.max_uses = cmd.max_uses;
        // TODO: Save to DB
        Ok(link_to_response(&link))
    }
    async fn get_link(&self, _link_id: Uuid) -> Result<PaymentLinkResponse, PlatformError> {
        Err(PlatformError::NotFound { resource: "PaymentLink".into(), id: _link_id })
    }
    async fn deactivate_link(&self, _link_id: Uuid) -> Result<(), PlatformError> { Ok(()) }
}

fn link_to_response(l: &PaymentLink) -> PaymentLinkResponse {
    PaymentLinkResponse { link_id: l.link_id, status: l.status.as_str().to_string(), amount: l.amount.amount_minor_units, currency: l.amount.currency.0.clone(), description: l.description.clone(), merchant_name: l.merchant_name.clone(), current_uses: l.current_uses, created_at: l.created_at.to_rfc3339() }
}

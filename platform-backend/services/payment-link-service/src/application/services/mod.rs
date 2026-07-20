use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::PaymentLink;
use crate::domain::rules::PaymentLinkRepository;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PaymentLinkServiceImpl { repo: Box<dyn PaymentLinkRepository>, db: DatabaseConnection }
impl PaymentLinkServiceImpl {
    pub fn new(repo: Box<dyn PaymentLinkRepository>, db: DatabaseConnection) -> Self { Self { repo, db } }
}

#[async_trait]
pub trait PaymentLinkService: Send + Sync {
    async fn create(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PlatformError>;
    async fn use_link(&self, cmd: UsePaymentLinkCommand) -> Result<PaymentLink, PlatformError>;
    async fn get_by_id(&self, id: Uuid) -> Result<PaymentLink, PlatformError>;
    async fn get_by_token(&self, token: &str) -> Result<PaymentLink, PlatformError>;
}

#[async_trait]
impl PaymentLinkService for PaymentLinkServiceImpl {
    async fn create(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PlatformError> {
        let amount = Money { amount_minor_units: cmd.amount_minor_units, currency: CurrencyCode::new(&cmd.currency).map_err(|_| PlatformError::Validation(platform_error::ValidationError::InvalidCurrencyCode))? };
        let expires_at = cmd.expires_in_hours.map(|h| chrono::Utc::now() + chrono::Duration::hours(h));
        let mut link = PaymentLink::new(cmd.operator_id, cmd.description, cmd.merchant_name, amount, cmd.max_uses, expires_at);
        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn use_link(&self, cmd: UsePaymentLinkCommand) -> Result<PaymentLink, PlatformError> {
        let mut link = self.repo.find_by_token(&cmd.public_token).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "payment_link".into(), id: Uuid::nil() })?;
        link.use_link().map_err(|e| PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition { from: "active".into(), command: e.into() }))?;
        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<PaymentLink, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "payment_link".into(), id })
    }

    async fn get_by_token(&self, token: &str) -> Result<PaymentLink, PlatformError> {
        self.repo.find_by_token(token).await?.ok_or_else(|| PlatformError::NotFound { resource: "payment_link".into(), id: Uuid::nil() })
    }
}

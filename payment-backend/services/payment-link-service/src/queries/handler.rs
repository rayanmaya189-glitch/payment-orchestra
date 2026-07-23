//! Payment Link query handlers — BC-07

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_payment_link(&self, id: Uuid) -> Result<PaymentLink, PaymentLinkError>;
    async fn get_payment_link_by_token(&self, token: &str) -> Result<PaymentLink, PaymentLinkError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError>;
    async fn find_expired_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError>;
}

pub struct PaymentLinkQueryHandler<R: PaymentLinkRepository> {
    repo: R,
}

impl<R: PaymentLinkRepository> PaymentLinkQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PaymentLinkRepository + Send + Sync> QueryHandler for PaymentLinkQueryHandler<R> {
    async fn get_payment_link(&self, id: Uuid) -> Result<PaymentLink, PaymentLinkError> {
        self.repo.load(id).await?.ok_or(PaymentLinkError::NotFound)
    }

    async fn get_payment_link_by_token(&self, token: &str) -> Result<PaymentLink, PaymentLinkError> {
        self.repo.load_by_token(token).await?.ok_or(PaymentLinkError::NotFound)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        self.repo.find_by_operator(operator_id).await
    }

    async fn find_expired_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        self.repo.find_expired().await
    }
}

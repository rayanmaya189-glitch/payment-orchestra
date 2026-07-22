//! Payment Link API surface — BC-07

use crate::commands::*;
use crate::domain::{PaymentLink, PaymentLinkError};
use crate::queries::*;
use uuid::Uuid;

/// Public API facade for the payment-link service.
pub struct PaymentLinkApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl PaymentLinkApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    pub async fn create_payment_link(
        &self,
        cmd: CreatePaymentLinkCommand,
    ) -> Result<PaymentLink, PaymentLinkError> {
        self.command_handler.create_payment_link(cmd).await
    }

    pub async fn resolve_payment_link(
        &self,
        cmd: ResolvePaymentLinkCommand,
    ) -> Result<PaymentLink, PaymentLinkError> {
        self.command_handler.resolve_payment_link(cmd).await
    }

    pub async fn cancel_payment_link(
        &self,
        cmd: CancelPaymentLinkCommand,
    ) -> Result<PaymentLink, PaymentLinkError> {
        self.command_handler.cancel_payment_link(cmd).await
    }

    pub async fn expire_overdue_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        self.command_handler.expire_overdue_links().await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_payment_link(&self, id: Uuid) -> Result<PaymentLink, PaymentLinkError> {
        self.query_handler.get_payment_link(id).await
    }

    pub async fn get_payment_link_by_token(&self, token: &str) -> Result<PaymentLink, PaymentLinkError> {
        self.query_handler.get_payment_link_by_token(token).await
    }

    pub async fn find_expired_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        self.query_handler.find_expired_links().await
    }
}

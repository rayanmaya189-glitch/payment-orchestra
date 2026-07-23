//! Subscription Billing API surface — BC-08

use crate::commands::*;
use crate::domain::{Subscription, SubscriptionError};
use crate::queries::*;
use uuid::Uuid;

/// Public API facade for the subscription service.
pub mod grpc;

pub struct SubscriptionApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl SubscriptionApi {
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

    pub async fn create_subscription(
        &self,
        cmd: CreateSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.create_subscription(cmd).await
    }

    pub async fn cancel_subscription(
        &self,
        cmd: CancelSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.cancel_subscription(cmd).await
    }

    pub async fn pause_subscription(
        &self,
        cmd: PauseSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.pause_subscription(cmd).await
    }

    pub async fn resume_subscription(
        &self,
        cmd: ResumeSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.resume_subscription(cmd).await
    }

    pub async fn renew_subscription(
        &self,
        cmd: RenewSubscriptionCommand,
    ) -> Result<RenewResult, SubscriptionError> {
        self.command_handler.renew_subscription(cmd).await
    }

    pub async fn confirm_renewal_payment(
        &self,
        cmd: ConfirmRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.confirm_renewal_payment(cmd).await
    }

    pub async fn fail_renewal_payment(
        &self,
        cmd: FailRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError> {
        self.command_handler.fail_renewal_payment(cmd).await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_subscription(&self, id: Uuid) -> Result<Subscription, SubscriptionError> {
        self.query_handler.get_subscription(id).await
    }

    pub async fn find_by_customer(
        &self,
        customer_id: Uuid,
    ) -> Result<Vec<Subscription>, SubscriptionError> {
        self.query_handler.find_by_customer(customer_id).await
    }

    pub async fn find_by_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<Subscription>, SubscriptionError> {
        self.query_handler.find_by_operator(operator_id).await
    }

    pub async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        self.query_handler.find_active_for_renewal().await
    }
}

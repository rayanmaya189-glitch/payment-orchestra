//! Command handler trait and implementation for subscription-service.

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;
use super::types::*;

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Subscription, SubscriptionError>;
    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<Subscription, SubscriptionError>;
    async fn pause_subscription(&self, cmd: PauseSubscriptionCommand) -> Result<Subscription, SubscriptionError>;
    async fn resume_subscription(&self, cmd: ResumeSubscriptionCommand) -> Result<Subscription, SubscriptionError>;
    async fn renew_subscription(&self, cmd: RenewSubscriptionCommand) -> Result<RenewResult, SubscriptionError>;
    async fn confirm_renewal_payment(&self, cmd: ConfirmRenewalPaymentCommand) -> Result<Subscription, SubscriptionError>;
    async fn fail_renewal_payment(&self, cmd: FailRenewalPaymentCommand) -> Result<Subscription, SubscriptionError>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct SubscriptionCommandHandler<R: SubscriptionRepository> {
    repo: R,
}

impl<R: SubscriptionRepository> SubscriptionCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SubscriptionRepository + Send + Sync> CommandHandler for SubscriptionCommandHandler<R> {
    async fn create_subscription(&self, cmd: CreateSubscriptionCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = Subscription::new(
            cmd.operator_id,
            cmd.customer_id,
            &cmd.plan,
            cmd.payment_method_token_id,
            cmd.max_dunning_retries.unwrap_or(3),
        )?;

        self.repo.save(&mut subscription).await?;
        Ok(subscription)
    }

    async fn cancel_subscription(&self, cmd: CancelSubscriptionCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.cancel(cmd.has_running_renewal)?;
        self.repo.save(&mut subscription).await?;
        Ok(subscription)
    }

    async fn pause_subscription(&self, cmd: PauseSubscriptionCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.pause()?;
        self.repo.save(&mut subscription).await?;
        Ok(subscription)
    }

    async fn resume_subscription(&self, cmd: ResumeSubscriptionCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.resume()?;
        self.repo.save(&mut subscription).await?;
        Ok(subscription)
    }

    async fn renew_subscription(&self, cmd: RenewSubscriptionCommand) -> Result<RenewResult, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        // Check if there's already a pending billing cycle
        if subscription.billing_cycles.iter().any(|c| c.status == BillingCycleStatus::Billing) {
            let cycle = subscription.billing_cycles.iter()
                .find(|c| c.status == BillingCycleStatus::Billing)
                .ok_or(SubscriptionError::BillingCycleNotFound)?;

            return Ok(RenewResult {
                subscription_id: cmd.subscription_id,
                billing_cycle_id: cycle.billing_cycle_id,
                idempotency_key: cycle.idempotency_key.clone(),
                amount_minor_units: subscription.plan_amount_minor_units,
                currency: subscription.currency.clone(),
            });
        }

        if subscription.status != SubscriptionStatus::Active
            && subscription.status != SubscriptionStatus::PastDue
        {
            return Err(SubscriptionError::InvalidTransition);
        }

        let cycle = subscription.start_billing_cycle()?;
        self.repo.save(&mut subscription).await?;

        Ok(RenewResult {
            subscription_id: cmd.subscription_id,
            billing_cycle_id: cycle.billing_cycle_id,
            idempotency_key: cycle.idempotency_key,
            amount_minor_units: subscription.plan_amount_minor_units,
            currency: subscription.currency.clone(),
        })
    }

    async fn confirm_renewal_payment(&self, cmd: ConfirmRenewalPaymentCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.record_successful_payment(cmd.billing_cycle_id, cmd.payment_intent_id)?;
        self.repo.save(&mut subscription).await?;
        Ok(subscription)
    }

    async fn fail_renewal_payment(&self, cmd: FailRenewalPaymentCommand) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self.repo.load(cmd.subscription_id).await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        match subscription.record_failed_payment(cmd.billing_cycle_id) {
            Ok(_) => {
                self.repo.save(&mut subscription).await?;
                Ok(subscription)
            }
            Err(SubscriptionError::DunningExhausted) => {
                self.repo.save(&mut subscription).await?;
                Ok(subscription)
            }
            Err(e) => Err(e),
        }
    }
}

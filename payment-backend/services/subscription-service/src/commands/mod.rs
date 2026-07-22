//! Subscription Billing command handlers — BC-08

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// Create a new subscription for a customer.
pub struct CreateSubscriptionCommand {
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub plan: SubscriptionPlan,
    pub payment_method_token_id: Option<Uuid>,
    pub max_dunning_retries: Option<i32>,
}

/// Cancel an active subscription.
/// Must check INV-SUB-01: no running renewal saga.
pub struct CancelSubscriptionCommand {
    pub subscription_id: Uuid,
    pub reason: Option<String>,
    /// Whether a renewal saga is currently running.
    pub has_running_renewal: bool,
}

/// Pause an active subscription.
pub struct PauseSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Resume a paused subscription.
pub struct ResumeSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Trigger a renewal (scheduled job).
pub struct RenewSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Record a successful payment for a renewal.
pub struct ConfirmRenewalPaymentCommand {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub payment_intent_id: Uuid,
}

/// Record a failed renewal payment (triggers dunning).
pub struct FailRenewalPaymentCommand {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Command results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RenewResult {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub idempotency_key: String,
    pub amount_minor_units: i64,
    pub currency: String,
}

// ---------------------------------------------------------------------------
// Command handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_subscription(
        &self,
        cmd: CreateSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError>;

    async fn cancel_subscription(
        &self,
        cmd: CancelSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError>;

    async fn pause_subscription(
        &self,
        cmd: PauseSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError>;

    async fn resume_subscription(
        &self,
        cmd: ResumeSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError>;

    async fn renew_subscription(
        &self,
        cmd: RenewSubscriptionCommand,
    ) -> Result<RenewResult, SubscriptionError>;

    async fn confirm_renewal_payment(
        &self,
        cmd: ConfirmRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError>;

    async fn fail_renewal_payment(
        &self,
        cmd: FailRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError>;
}

// ---------------------------------------------------------------------------
// Handler implementation
// ---------------------------------------------------------------------------

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
    async fn create_subscription(
        &self,
        cmd: CreateSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let subscription = Subscription::new(
            cmd.operator_id,
            cmd.customer_id,
            &cmd.plan,
            cmd.payment_method_token_id,
            cmd.max_dunning_retries.unwrap_or(3),
        )?;

        self.repo.save(&subscription).await?;
        Ok(subscription)
    }

    async fn cancel_subscription(
        &self,
        cmd: CancelSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.cancel(cmd.has_running_renewal)?;
        self.repo.save(&subscription).await?;
        Ok(subscription)
    }

    async fn pause_subscription(
        &self,
        cmd: PauseSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.pause()?;
        self.repo.save(&subscription).await?;
        Ok(subscription)
    }

    async fn resume_subscription(
        &self,
        cmd: ResumeSubscriptionCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.resume()?;
        self.repo.save(&subscription).await?;
        Ok(subscription)
    }

    async fn renew_subscription(
        &self,
        cmd: RenewSubscriptionCommand,
    ) -> Result<RenewResult, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        // Check if there's already a pending billing cycle
        if subscription
            .billing_cycles
            .iter()
            .any(|c| c.status == BillingCycleStatus::Billing)
        {
            // Idempotency: return existing pending cycle info
            let cycle = subscription
                .billing_cycles
                .iter()
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

        // Only renew if active or past_due
        if subscription.status != SubscriptionStatus::Active
            && subscription.status != SubscriptionStatus::PastDue
        {
            return Err(SubscriptionError::InvalidTransition);
        }

        let cycle = subscription.start_billing_cycle()?;
        self.repo.save(&subscription).await?;

        Ok(RenewResult {
            subscription_id: cmd.subscription_id,
            billing_cycle_id: cycle.billing_cycle_id,
            idempotency_key: cycle.idempotency_key,
            amount_minor_units: subscription.plan_amount_minor_units,
            currency: subscription.currency.clone(),
        })
    }

    async fn confirm_renewal_payment(
        &self,
        cmd: ConfirmRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        subscription.record_successful_payment(cmd.billing_cycle_id, cmd.payment_intent_id)?;
        self.repo.save(&subscription).await?;
        Ok(subscription)
    }

    async fn fail_renewal_payment(
        &self,
        cmd: FailRenewalPaymentCommand,
    ) -> Result<Subscription, SubscriptionError> {
        let mut subscription = self
            .repo
            .load(cmd.subscription_id)
            .await?
            .ok_or(SubscriptionError::NotFound(cmd.subscription_id))?;

        match subscription.record_failed_payment(cmd.billing_cycle_id) {
            Ok(_) => {
                self.repo.save(&subscription).await?;
                Ok(subscription)
            }
            Err(SubscriptionError::DunningExhausted) => {
                // Dunning exhausted transitions to cancelled
                self.repo.save(&subscription).await?;
                Ok(subscription)
            }
            Err(e) => Err(e),
        }
    }
}

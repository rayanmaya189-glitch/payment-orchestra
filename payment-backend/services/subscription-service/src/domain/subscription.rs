//! Subscription aggregate root. Event-sourced (BC-08).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::billing_cycle::{BillingCycle, BillingCycleStatus};
use super::dunning::{DunningRetry, DunningStatus};
use super::error::SubscriptionError;
use super::plan::SubscriptionPlan;
use super::subscription_status::SubscriptionStatus;

/// Core Subscription aggregate root. Event-sourced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: String,
    pub plan_amount_minor_units: i64,
    pub currency: String,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub billing_interval_days: i64,
    pub payment_method_token_id: Option<Uuid>,
    pub dunning_retry_count: i32,
    pub max_dunning_retries: i32,
    pub billing_cycles: Vec<BillingCycle>,
    pub dunning_retries: Vec<DunningRetry>,
    pub created_at: DateTime<Utc>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub paused_at: Option<DateTime<Utc>>,
    pub resumed_at: Option<DateTime<Utc>>,
}

impl Subscription {
    /// Create a new subscription in `Active` status.
    pub fn new(
        operator_id: Uuid,
        customer_id: Uuid,
        plan: &SubscriptionPlan,
        payment_method_token_id: Option<Uuid>,
        max_dunning_retries: i32,
    ) -> Result<Self, SubscriptionError> {
        if plan.amount_minor_units <= 0 {
            return Err(SubscriptionError::InvalidPlanAmount);
        }

        let now = Utc::now();
        let trial_days = plan.trial_period_days.unwrap_or(0);
        let period_start = now + Duration::days(trial_days);
        let period_end = period_start + Duration::days(plan.billing_interval_days);

        let initial_cycle = BillingCycle {
            billing_cycle_id: Uuid::now_v7(),
            period_start,
            period_end,
            status: BillingCycleStatus::Pending,
            payment_intent_id: None,
            idempotency_key: format!("{}:{}", Uuid::now_v7(), 0),
            created_at: now,
        };

        Ok(Self {
            subscription_id: Uuid::now_v7(),
            operator_id,
            customer_id,
            plan_id: plan.plan_id.clone(),
            plan_amount_minor_units: plan.amount_minor_units,
            currency: plan.currency.clone(),
            status: SubscriptionStatus::Active,
            current_period_start: period_start,
            current_period_end: period_end,
            billing_interval_days: plan.billing_interval_days,
            payment_method_token_id,
            dunning_retry_count: 0,
            max_dunning_retries: max_dunning_retries.max(1),
            billing_cycles: vec![initial_cycle],
            dunning_retries: Vec::new(),
            created_at: now,
            cancelled_at: None,
            paused_at: None,
            resumed_at: None,
        })
    }

    /// Cancel this subscription.
    /// INV-SUB-01: Cannot cancel while a renewal is in progress.
    pub fn cancel(&mut self, has_running_renewal: bool) -> Result<(), SubscriptionError> {
        if self.status == SubscriptionStatus::Cancelled {
            return Err(SubscriptionError::AlreadyCancelled);
        }
        if has_running_renewal {
            return Err(SubscriptionError::CannotCancelDuringRenewal);
        }
        if !self.status.can_transition_to(&SubscriptionStatus::Cancelled) {
            return Err(SubscriptionError::InvalidTransition);
        }
        self.status = SubscriptionStatus::Cancelled;
        self.cancelled_at = Some(Utc::now());
        Ok(())
    }

    /// Pause this subscription (manual hold).
    pub fn pause(&mut self) -> Result<(), SubscriptionError> {
        if self.status != SubscriptionStatus::Active {
            return Err(SubscriptionError::InvalidTransition);
        }
        self.status = SubscriptionStatus::Paused;
        self.paused_at = Some(Utc::now());
        Ok(())
    }

    /// Resume a paused subscription.
    pub fn resume(&mut self) -> Result<(), SubscriptionError> {
        if self.status != SubscriptionStatus::Paused {
            return Err(SubscriptionError::InvalidTransition);
        }
        self.status = SubscriptionStatus::Active;
        self.resumed_at = Some(Utc::now());
        // Adjust period end to account for pause duration
        if let Some(paused_at) = self.paused_at {
            let pause_duration = Utc::now() - paused_at;
            self.current_period_end += pause_duration;
        }
        Ok(())
    }

    /// Start a new billing cycle (for renewal).
    pub fn start_billing_cycle(&mut self) -> Result<BillingCycle, SubscriptionError> {
        if self.status != SubscriptionStatus::Active && self.status != SubscriptionStatus::PastDue {
            return Err(SubscriptionError::InvalidTransition);
        }

        let now = Utc::now();
        let period_start = self.current_period_end;
        let period_end = period_start + Duration::days(self.billing_interval_days);
        let cycle_number = self.billing_cycles.len() as i64;

        let cycle = BillingCycle {
            billing_cycle_id: Uuid::now_v7(),
            period_start,
            period_end,
            status: BillingCycleStatus::Billing,
            payment_intent_id: None,
            idempotency_key: format!("{}:{}", self.subscription_id, cycle_number),
            created_at: now,
        };

        self.current_period_start = period_start;
        self.current_period_end = period_end;
        self.billing_cycles.push(cycle.clone());
        Ok(cycle)
    }

    /// Record a successful payment for a billing cycle.
    pub fn record_successful_payment(
        &mut self,
        billing_cycle_id: Uuid,
        payment_intent_id: Uuid,
    ) -> Result<(), SubscriptionError> {
        let cycle = self
            .billing_cycles
            .iter_mut()
            .find(|c| c.billing_cycle_id == billing_cycle_id)
            .ok_or(SubscriptionError::BillingCycleNotFound)?;

        cycle.status = BillingCycleStatus::Succeeded;
        cycle.payment_intent_id = Some(payment_intent_id);

        // If was past_due, return to active
        if self.status == SubscriptionStatus::PastDue {
            self.status = SubscriptionStatus::Active;
            self.dunning_retry_count = 0;
            self.dunning_retries.clear();
        }

        Ok(())
    }

    /// Record a failed payment (triggers dunning).
    pub fn record_failed_payment(
        &mut self,
        billing_cycle_id: Uuid,
    ) -> Result<DunningRetry, SubscriptionError> {
        let cycle = self
            .billing_cycles
            .iter_mut()
            .find(|c| c.billing_cycle_id == billing_cycle_id)
            .ok_or(SubscriptionError::BillingCycleNotFound)?;

        cycle.status = BillingCycleStatus::Failed;

        // Transition to past_due if active
        if self.status == SubscriptionStatus::Active {
            self.status = SubscriptionStatus::PastDue;
        }

        // Schedule next retry
        self.dunning_retry_count += 1;
        if self.dunning_retry_count >= self.max_dunning_retries {
            self.status = SubscriptionStatus::Cancelled;
            self.cancelled_at = Some(Utc::now());
            return Err(SubscriptionError::DunningExhausted);
        }

        // Retry schedule: day 1, 3, 7, ...
        let retry_delay_days = match self.dunning_retry_count {
            1 => 1,
            2 => 3,
            3 => 7,
            n => 7 * (n - 2), // linear backoff after day 7
        };

        let retry = DunningRetry {
            retry_number: self.dunning_retry_count,
            scheduled_at: Utc::now() + Duration::days(retry_delay_days as i64),
            attempted_at: None,
            status: DunningStatus::Pending,
        };

        self.dunning_retries.push(retry.clone());
        Ok(retry)
    }

    /// Check if a dunning retry is due now.
    pub fn is_dunning_due(&self) -> bool {
        if self.status != SubscriptionStatus::PastDue {
            return false;
        }
        self.dunning_retries
            .iter()
            .any(|r| r.status == DunningStatus::Pending && Utc::now() >= r.scheduled_at)
    }
}

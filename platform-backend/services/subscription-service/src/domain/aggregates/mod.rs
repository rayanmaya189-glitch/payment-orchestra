use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;

use crate::domain::value_objects::{DunningConfig, SubscriptionInterval, SubscriptionStatus};

/// Subscription aggregate root.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub status: SubscriptionStatus,
    pub amount: Money,
    pub interval: SubscriptionInterval,
    pub interval_count: i32,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_period_days: i32,
    pub payment_method_token_id: Option<String>,
    pub failed_payment_intent_id: Option<Uuid>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub canceled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uncommitted_events: Vec<SubscriptionEvent>,
}

#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    Created {
        customer_id: Uuid,
        amount_minor_units: i64,
        currency: String,
        interval: String,
    },
    PaymentSucceeded {
        payment_intent_id: Option<Uuid>,
        period_end: DateTime<Utc>,
    },
    PaymentFailed {
        payment_intent_id: Option<Uuid>,
        reason: String,
    },
    PastDue {
        retry_count: i32,
        next_retry_at: DateTime<Utc>,
    },
    Canceled {
        reason: String,
    },
    Reactivated,
    PaymentMethodUpdated {
        old_token: Option<String>,
        new_token: String,
    },
}

impl Subscription {
    pub fn new(
        operator_id: Uuid,
        customer_id: Uuid,
        amount: Money,
        interval: SubscriptionInterval,
        interval_count: i32,
        trial_period_days: i32,
    ) -> Self {
        let now = Utc::now();
        let period_end = now + interval.duration(interval_count);

        let mut sub = Self {
            subscription_id: Uuid::now_v7(),
            operator_id,
            customer_id,
            status: if trial_period_days > 0 {
                SubscriptionStatus::Trialing
            } else {
                SubscriptionStatus::Active
            },
            amount,
            interval,
            interval_count,
            current_period_start: now,
            current_period_end: period_end,
            trial_period_days,
            payment_method_token_id: None,
            failed_payment_intent_id: None,
            retry_count: 0,
            max_retries: 3,
            canceled_at: None,
            created_at: now,
            updated_at: now,
            uncommitted_events: Vec::new(),
        };

        sub.apply(SubscriptionEvent::Created {
            customer_id,
            amount_minor_units: sub.amount.amount_minor_units,
            currency: sub.amount.currency.0.clone(),
            interval: sub.interval.as_str().to_string(),
        });

        sub
    }

    pub fn apply(&mut self, event: SubscriptionEvent) {
        match &event {
            SubscriptionEvent::PaymentSucceeded {
                payment_intent_id,
                period_end,
            } => {
                self.status = SubscriptionStatus::Active;
                self.current_period_start = self.current_period_end;
                self.current_period_end = *period_end;
                self.retry_count = 0;
                self.failed_payment_intent_id = *payment_intent_id;
            }
            SubscriptionEvent::PaymentFailed {
                payment_intent_id,
                ..
            } => {
                self.failed_payment_intent_id = *payment_intent_id;
                self.retry_count += 1;
                if self.retry_count >= self.max_retries {
                    self.status = SubscriptionStatus::Canceled;
                } else {
                    self.status = SubscriptionStatus::PastDue;
                }
            }
            SubscriptionEvent::Canceled { .. } => {
                self.status = SubscriptionStatus::Canceled;
                self.canceled_at = Some(Utc::now());
            }
            SubscriptionEvent::Reactivated => {
                self.status = SubscriptionStatus::Active;
                self.canceled_at = None;
                self.retry_count = 0;
                self.failed_payment_intent_id = None;
            }
            SubscriptionEvent::PaymentMethodUpdated { new_token, .. } => {
                self.payment_method_token_id = Some(new_token.clone());
            }
            SubscriptionEvent::PastDue { .. } => {
                self.status = SubscriptionStatus::PastDue;
            }
            _ => {}
        }
        self.updated_at = Utc::now();
        self.uncommitted_events.push(event);
    }

    pub fn take_uncommitted_events(&mut self) -> Vec<SubscriptionEvent> {
        std::mem::take(&mut self.uncommitted_events)
    }

    pub fn can_charge(&self) -> bool {
        matches!(
            self.status,
            SubscriptionStatus::Active | SubscriptionStatus::PastDue | SubscriptionStatus::Trialing
        )
    }

    pub fn record_payment_success(&mut self, payment_intent_id: Option<Uuid>) {
        let period_end = self.current_period_end + self.interval.duration(self.interval_count);
        self.apply(SubscriptionEvent::PaymentSucceeded {
            payment_intent_id,
            period_end,
        });
    }

    pub fn record_payment_failure(
        &mut self,
        payment_intent_id: Option<Uuid>,
        reason: &str,
    ) {
        self.apply(SubscriptionEvent::PaymentFailed {
            payment_intent_id,
            reason: reason.to_string(),
        });
    }

    pub fn cancel(&mut self, reason: &str) {
        self.apply(SubscriptionEvent::Canceled {
            reason: reason.to_string(),
        });
    }

    /// Reactivate a canceled subscription.
    /// Returns Err if the subscription is not in a reactivatable state.
    pub fn reactivate(&mut self) -> Result<(), &'static str> {
        match self.status {
            SubscriptionStatus::Canceled => {
                self.apply(SubscriptionEvent::Reactivated);
                Ok(())
            }
            SubscriptionStatus::Active => Err("Subscription is already active"),
            SubscriptionStatus::Trialing => Err("Subscription is still in trial"),
            SubscriptionStatus::PastDue => {
                // PastDue subscriptions don't need reactivation, just payment
                Err("Subscription is past due, retry payment instead")
            }
            SubscriptionStatus::Unpaid => {
                self.apply(SubscriptionEvent::Reactivated);
                Ok(())
            }
        }
    }

    /// Update the payment method token.
    pub fn update_payment_method(&mut self, new_token: String) {
        let old_token = self.payment_method_token_id.clone();
        self.apply(SubscriptionEvent::PaymentMethodUpdated {
            old_token,
            new_token,
        });
    }

    /// Transition to past due status with dunning schedule.
    pub fn mark_past_due(&mut self, dunning_config: &DunningConfig) {
        let next_retry_at =
            Utc::now() + dunning_config.delay_for_attempt(self.retry_count);
        self.apply(SubscriptionEvent::PastDue {
            retry_count: self.retry_count,
            next_retry_at,
        });
    }

    /// Check if the subscription is past its current period end.
    pub fn is_past_due_date(&self) -> bool {
        Utc::now() > self.current_period_end
    }

    /// Check if the subscription is within its trial period.
    pub fn is_in_trial(&self) -> bool {
        self.status == SubscriptionStatus::Trialing
    }

    /// Check if the subscription can be reactivated.
    pub fn can_reactivate(&self) -> bool {
        matches!(
            self.status,
            SubscriptionStatus::Canceled | SubscriptionStatus::Unpaid
        )
    }

    /// Validate the subscription state for charging.
    pub fn validate_for_charge(&self) -> Result<(), &'static str> {
        if !self.can_charge() {
            return Err("Subscription cannot be charged in current status");
        }
        if self.payment_method_token_id.is_none() {
            return Err("No payment method attached");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money {
            amount_minor_units: amount,
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
        }
    }

    fn test_sub() -> Subscription {
        Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            aed(1000),
            SubscriptionInterval::Monthly,
            1,
            0,
        )
    }

    #[test]
    fn test_new_subscription() {
        let sub = test_sub();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert_eq!(sub.retry_count, 0);
        assert_eq!(sub.max_retries, 3);
        assert!(sub.can_charge());
    }

    #[test]
    fn test_new_subscription_with_trial() {
        let sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            aed(1000),
            SubscriptionInterval::Monthly,
            1,
            14,
        );
        assert_eq!(sub.status, SubscriptionStatus::Trialing);
        assert!(sub.is_in_trial());
    }

    #[test]
    fn test_payment_success_advances_period() {
        let mut sub = test_sub();
        let old_end = sub.current_period_end;
        let intent_id = Some(Uuid::now_v7());
        sub.record_payment_success(intent_id);
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.current_period_end > old_end);
        assert_eq!(sub.failed_payment_intent_id, intent_id);
    }

    #[test]
    fn test_payment_failure_increments_retry() {
        let mut sub = test_sub();
        sub.record_payment_failure(None, "insufficient funds");
        assert_eq!(sub.status, SubscriptionStatus::PastDue);
        assert_eq!(sub.retry_count, 1);
    }

    #[test]
    fn test_max_retries_cancels() {
        let mut sub = test_sub();
        sub.max_retries = 2;
        sub.record_payment_failure(None, "fail 1");
        sub.record_payment_failure(None, "fail 2");
        assert_eq!(sub.status, SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_cancel() {
        let mut sub = test_sub();
        sub.cancel("user request");
        assert_eq!(sub.status, SubscriptionStatus::Canceled);
        assert!(sub.canceled_at.is_some());
    }

    #[test]
    fn test_reactivate_canceled() {
        let mut sub = test_sub();
        sub.cancel("user request");
        assert!(sub.reactivate().is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.canceled_at.is_none());
        assert_eq!(sub.retry_count, 0);
    }

    #[test]
    fn test_reactivate_already_active() {
        let mut sub = test_sub();
        assert_eq!(sub.reactivate(), Err("Subscription is already active"));
    }

    #[test]
    fn test_reactivate_unpaid() {
        let mut sub = test_sub();
        sub.status = SubscriptionStatus::Unpaid;
        assert!(sub.reactivate().is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Active);
    }

    #[test]
    fn test_reactivate_past_due_fails() {
        let mut sub = test_sub();
        sub.status = SubscriptionStatus::PastDue;
        assert_eq!(
            sub.reactivate(),
            Err("Subscription is past due, retry payment instead")
        );
    }

    #[test]
    fn test_update_payment_method() {
        let mut sub = test_sub();
        assert!(sub.payment_method_token_id.is_none());
        sub.update_payment_method("tok_new_123".to_string());
        assert_eq!(
            sub.payment_method_token_id,
            Some("tok_new_123".to_string())
        );
    }

    #[test]
    fn test_mark_past_due() {
        let mut sub = test_sub();
        let dunning = DunningConfig::standard();
        sub.mark_past_due(&dunning);
        assert_eq!(sub.status, SubscriptionStatus::PastDue);
    }

    #[test]
    fn test_is_past_due_date() {
        let mut sub = test_sub();
        assert!(!sub.is_past_due_date());
        // Simulate past period end
        sub.current_period_end = Utc::now() - chrono::Duration::hours(1);
        assert!(sub.is_past_due_date());
    }

    #[test]
    fn test_validate_for_charge() {
        let mut sub = test_sub();
        assert_eq!(
            sub.validate_for_charge(),
            Err("No payment method attached")
        );
        sub.payment_method_token_id = Some("tok_123".to_string());
        assert!(sub.validate_for_charge().is_ok());
    }

    #[test]
    fn test_validate_for_charge_canceled() {
        let mut sub = test_sub();
        sub.cancel("test");
        assert_eq!(
            sub.validate_for_charge(),
            Err("Subscription cannot be charged in current status")
        );
    }

    #[test]
    fn test_can_reactivate() {
        let mut sub = test_sub();
        assert!(!sub.can_reactivate());
        sub.cancel("test");
        assert!(sub.can_reactivate());
        sub.status = SubscriptionStatus::Unpaid;
        assert!(sub.can_reactivate());
    }

    #[test]
    fn test_uncommitted_events() {
        let mut sub = test_sub();
        sub.record_payment_success(None);
        sub.cancel("test");
        let events = sub.take_uncommitted_events();
        assert_eq!(events.len(), 3); // Created + PaymentSucceeded + Canceled
        assert!(sub.uncommitted_events.is_empty());
    }

    #[test]
    fn test_payment_failure_records_intent_id() {
        let mut sub = test_sub();
        let intent_id = Some(Uuid::now_v7());
        sub.record_payment_failure(intent_id, "declined");
        assert_eq!(sub.failed_payment_intent_id, intent_id);
    }

    #[test]
    fn test_dunning_config() {
        let config = DunningConfig::standard();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_intervals.len(), 5);
    }
}

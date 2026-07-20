use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;

use crate::domain::value_objects::{SubscriptionInterval, SubscriptionStatus};

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
        period_end: DateTime<Utc>,
    },
    PaymentFailed {
        reason: String,
    },
    PastDue {
        retry_count: i32,
    },
    Canceled {
        reason: String,
    },
    Reactivated,
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
            status: if trial_period_days > 0 { SubscriptionStatus::Trialing } else { SubscriptionStatus::Active },
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
            SubscriptionEvent::PaymentSucceeded { period_end } => {
                self.status = SubscriptionStatus::Active;
                self.current_period_start = self.current_period_end;
                self.current_period_end = *period_end;
                self.retry_count = 0;
                self.failed_payment_intent_id = None;
            }
            SubscriptionEvent::PaymentFailed { .. } => {
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
                if self.status == SubscriptionStatus::Canceled {
                    self.status = SubscriptionStatus::Active;
                    self.canceled_at = None;
                    self.retry_count = 0;
                }
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
        matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::PastDue | SubscriptionStatus::Trialing)
    }

    pub fn record_payment_success(&mut self) {
        let period_end = self.current_period_end + self.interval.duration(self.interval_count);
        self.apply(SubscriptionEvent::PaymentSucceeded { period_end });
    }

    pub fn record_payment_failure(&mut self, reason: &str) {
        self.apply(SubscriptionEvent::PaymentFailed {
            reason: reason.to_string(),
        });
    }

    pub fn cancel(&mut self, reason: &str) {
        self.apply(SubscriptionEvent::Canceled {
            reason: reason.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() }
    }

    #[test]
    fn test_new_subscription() {
        let sub = Subscription::new(
            Uuid::now_v7(), Uuid::now_v7(), aed(1000),
            SubscriptionInterval::Monthly, 1, 0,
        );
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert_eq!(sub.retry_count, 0);
    }

    #[test]
    fn test_payment_success_advances_period() {
        let mut sub = Subscription::new(
            Uuid::now_v7(), Uuid::now_v7(), aed(1000),
            SubscriptionInterval::Monthly, 1, 0,
        );
        let old_end = sub.current_period_end;
        sub.record_payment_success();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.current_period_end > old_end);
    }

    #[test]
    fn test_payment_failure_increments_retry() {
        let mut sub = Subscription::new(
            Uuid::now_v7(), Uuid::now_v7(), aed(1000),
            SubscriptionInterval::Monthly, 1, 0,
        );
        sub.record_payment_failure("insufficient funds");
        assert_eq!(sub.status, SubscriptionStatus::PastDue);
        assert_eq!(sub.retry_count, 1);
    }

    #[test]
    fn test_max_retries_cancels() {
        let mut sub = Subscription::new(
            Uuid::now_v7(), Uuid::now_v7(), aed(1000),
            SubscriptionInterval::Monthly, 1, 0,
        );
        sub.max_retries = 2;
        sub.record_payment_failure("fail 1");
        sub.record_payment_failure("fail 2");
        assert_eq!(sub.status, SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_cancel() {
        let mut sub = Subscription::new(
            Uuid::now_v7(), Uuid::now_v7(), aed(1000),
            SubscriptionInterval::Monthly, 1, 0,
        );
        sub.cancel("user request");
        assert_eq!(sub.status, SubscriptionStatus::Canceled);
        assert!(sub.canceled_at.is_some());
    }
}

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{BillingInterval, SubscriptionStatus};
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct Subscription {
    pub subscription_id: Uuid, pub operator_id: Uuid, pub customer_id: Uuid, pub status: SubscriptionStatus,
    pub amount: Money, pub interval: BillingInterval, pub current_period_start: DateTime<Utc>, pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
impl Subscription {
    pub fn new(operator_id: Uuid, customer_id: Uuid, amount: Money, interval: BillingInterval) -> Self {
        let now = Utc::now();
        let period_end = match interval { BillingInterval::Monthly => now + chrono::Duration::days(30), BillingInterval::Quarterly => now + chrono::Duration::days(90), BillingInterval::Yearly => now + chrono::Duration::days(365) };
        Self { subscription_id: Uuid::now_v7(), operator_id, customer_id, status: SubscriptionStatus::Active, amount, interval, current_period_start: now, current_period_end: period_end, created_at: now }
    }
    pub fn can_pause(&self) -> bool { self.status == SubscriptionStatus::Active }
    pub fn can_resume(&self) -> bool { self.status == SubscriptionStatus::Paused }
    pub fn can_cancel(&self) -> bool { matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Paused) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() }
    }

    #[test]
    fn test_new_subscription_is_active() {
        let s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Monthly);
        assert_eq!(s.status, SubscriptionStatus::Active);
        assert!(s.can_pause());
        assert!(!s.can_resume());
        assert!(s.can_cancel());
    }

    #[test]
    fn test_period_end_monthly() {
        let s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Monthly);
        let diff = s.current_period_end - s.current_period_start;
        assert!(diff.num_days() >= 29 && diff.num_days() <= 30);
    }

    #[test]
    fn test_period_end_yearly() {
        let s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Yearly);
        let diff = s.current_period_end - s.current_period_start;
        assert_eq!(diff.num_days(), 365);
    }

    #[test]
    fn test_can_pause_only_when_active() {
        let mut s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Monthly);
        assert!(s.can_pause());
        s.status = SubscriptionStatus::Paused;
        assert!(!s.can_pause());
    }

    #[test]
    fn test_can_resume_only_when_paused() {
        let mut s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Monthly);
        assert!(!s.can_resume());
        s.status = SubscriptionStatus::Paused;
        assert!(s.can_resume());
    }

    #[test]
    fn test_cannot_cancel_when_cancelled() {
        let mut s = Subscription::new(Uuid::now_v7(), Uuid::now_v7(), aed(10000), BillingInterval::Monthly);
        assert!(s.can_cancel());
        s.status = SubscriptionStatus::Cancelled;
        assert!(!s.can_cancel());
    }
}

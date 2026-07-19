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

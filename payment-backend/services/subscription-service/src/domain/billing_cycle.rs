//! BillingCycle entity within Subscription aggregate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a single billing period within a subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingCycle {
    pub billing_cycle_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub status: BillingCycleStatus,
    pub payment_intent_id: Option<Uuid>,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingCycleStatus {
    Pending,
    Billing,
    Succeeded,
    Failed,
}

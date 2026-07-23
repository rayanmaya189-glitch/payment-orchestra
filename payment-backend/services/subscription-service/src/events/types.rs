//! Event type definitions for subscription-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Event type string constants ─────────────────────────────────────────────

pub const EVENT_TYPE_CREATED: &str = "subscription.created";
pub const EVENT_TYPE_CANCELLED: &str = "subscription.cancelled";
pub const EVENT_TYPE_PAUSED: &str = "subscription.paused";
pub const EVENT_TYPE_RESUMED: &str = "subscription.resumed";
pub const EVENT_TYPE_RENEWAL_STARTED: &str = "subscription.renewal_started";
pub const EVENT_TYPE_RENEWAL_SUCCEEDED: &str = "subscription.renewal_succeeded";
pub const EVENT_TYPE_RENEWAL_FAILED: &str = "subscription.renewal_failed";
pub const EVENT_TYPE_DUNNING_ATTEMPTED: &str = "subscription.dunning_attempted";
pub const EVENT_TYPE_DUNNING_EXHAUSTED: &str = "subscription.dunning_exhausted";

// ─── Event enum ──────────────────────────────────────────────────────────────

/// All events that a Subscription aggregate can produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionEvent {
    Created(SubscriptionCreated),
    Cancelled(SubscriptionCancelled),
    Paused(SubscriptionPaused),
    Resumed(SubscriptionResumed),
    RenewalStarted(SubscriptionRenewalStarted),
    RenewalSucceeded(SubscriptionRenewalSucceeded),
    RenewalFailed(SubscriptionRenewalFailed),
    DunningAttempted(SubscriptionDunningAttempted),
    DunningExhausted(SubscriptionDunningExhausted),
}

impl SubscriptionEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => EVENT_TYPE_CREATED,
            Self::Cancelled(_) => EVENT_TYPE_CANCELLED,
            Self::Paused(_) => EVENT_TYPE_PAUSED,
            Self::Resumed(_) => EVENT_TYPE_RESUMED,
            Self::RenewalStarted(_) => EVENT_TYPE_RENEWAL_STARTED,
            Self::RenewalSucceeded(_) => EVENT_TYPE_RENEWAL_SUCCEEDED,
            Self::RenewalFailed(_) => EVENT_TYPE_RENEWAL_FAILED,
            Self::DunningAttempted(_) => EVENT_TYPE_DUNNING_ATTEMPTED,
            Self::DunningExhausted(_) => EVENT_TYPE_DUNNING_EXHAUSTED,
        }
    }
}

// ─── Event payloads ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionCreated {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub payment_method_token_id: Option<Uuid>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionCancelled {
    pub subscription_id: Uuid,
    pub reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPaused {
    pub subscription_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResumed {
    pub subscription_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRenewalStarted {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRenewalSucceeded {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub payment_intent_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRenewalFailed {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionDunningAttempted {
    pub subscription_id: Uuid,
    pub retry_number: i32,
    pub scheduled_at: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionDunningExhausted {
    pub subscription_id: Uuid,
    pub max_retries: i32,
    pub occurred_at: DateTime<Utc>,
}

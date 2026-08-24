//! SubscriptionPlan value object / reference data.

use serde::{Deserialize, Serialize};

/// A billing plan that subscriptions are based on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub plan_id: String,
    pub name: String,
    pub amount_minor_units: i64,
    pub currency: String,
    /// Billing interval in days (e.g., 30 for monthly, 365 for yearly).
    pub billing_interval_days: i64,
    /// Optional trial period in days.
    pub trial_period_days: Option<i64>,
    pub is_active: bool,
}

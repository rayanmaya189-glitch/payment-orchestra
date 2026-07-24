//! SubscriptionStatus state machine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    /// Subscription is active and billing normally.
    Active,
    /// Payment failed, currently in dunning (retry) period.
    PastDue,
    /// Subscription has been cancelled.
    Cancelled,
    /// Subscription is paused (manual hold).
    Paused,
}

impl SubscriptionStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use SubscriptionStatus::*;
        matches!(
            (self, target),
            (Active, PastDue)          // payment failure → dunning
                | (Active, Cancelled)  // manual cancel
                | (Active, Paused)     // manual pause
                | (PastDue, Active)    // successful retry
                | (PastDue, Cancelled) // dunning exhausted or manual cancel
                | (Paused, Active)     // manual resume
        )
    }
}

impl std::str::FromStr for SubscriptionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(SubscriptionStatus::Active),
            "past_due" => Ok(SubscriptionStatus::PastDue),
            "cancelled" => Ok(SubscriptionStatus::Cancelled),
            "paused" => Ok(SubscriptionStatus::Paused),
            _ => Err(format!("Invalid subscription status: {}", s)),
        }
    }
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::PastDue => write!(f, "past_due"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Paused => write!(f, "paused"),
        }
    }
}

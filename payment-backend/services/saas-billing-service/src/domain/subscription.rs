//! Tenant Subscription domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::SaaSbillingError;

/// A merchant's subscription to a SaaS plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSubscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub payment_method_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Subscription status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Unpaid,
    Paused,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Unpaid => "unpaid",
            Self::Paused => "paused",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "trialing" => Some(Self::Trialing),
            "active" => Some(Self::Active),
            "past_due" => Some(Self::PastDue),
            "canceled" => Some(Self::Canceled),
            "unpaid" => Some(Self::Unpaid),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }

    /// Check if the subscription allows access.
    pub fn allows_access(&self) -> bool {
        matches!(self, Self::Trialing | Self::Active | Self::PastDue)
    }

    /// Check if the status can transition to another status.
    pub fn can_transition_to(&self, target: &SubscriptionStatus) -> bool {
        matches!(
            (self, target),
            // Trial transitions
            (Self::Trialing, Self::Active)
                | (Self::Trialing, Self::Canceled)
                // Active transitions
                | (Self::Active, Self::PastDue)
                | (Self::Active, Self::Canceled)
                | (Self::Active, Self::Paused)
                // PastDue transitions
                | (Self::PastDue, Self::Active)
                | (Self::PastDue, Self::Canceled)
                | (Self::PastDue, Self::Unpaid)
                // Paused transitions
                | (Self::Paused, Self::Active)
                | (Self::Paused, Self::Canceled)
                // Unpaid transitions
                | (Self::Unpaid, Self::Active)
                | (Self::Unpaid, Self::Canceled)
        )
    }
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SubscriptionStatus {
    type Err = SaaSbillingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).ok_or_else(|| SaaSbillingError::InvalidStatusTransition {
            from: "unknown".into(),
            to: s.into(),
        })
    }
}

impl TenantSubscription {
    /// Create a new subscription.
    pub fn new(
        operator_id: Uuid,
        plan_id: Uuid,
        created_by: Uuid,
        trial_days: Option<i32>,
    ) -> Self {
        let now = Utc::now();
        let (status, trial_ends_at) = if let Some(days) = trial_days {
            (SubscriptionStatus::Trialing, Some(now + chrono::Duration::days(days as i64)))
        } else {
            (SubscriptionStatus::Active, None)
        };

        Self {
            subscription_id: Uuid::now_v7(),
            operator_id,
            plan_id,
            status,
            current_period_start: now,
            current_period_end: now + chrono::Duration::days(30),
            trial_ends_at,
            canceled_at: None,
            cancel_reason: None,
            payment_method_id: None,
            stripe_subscription_id: None,
            created_by,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the subscription is currently active (including trial).
    pub fn is_active(&self) -> bool {
        self.status.allows_access()
    }

    /// Check if the subscription is in trial period.
    pub fn is_trialing(&self) -> bool {
        self.status == SubscriptionStatus::Trialing
    }

    /// Check if the trial has expired.
    pub fn is_trial_expired(&self) -> bool {
        if let Some(trial_end) = self.trial_ends_at {
            Utc::now() > trial_end
        } else {
            false
        }
    }

    /// Check if the current period has ended.
    pub fn is_period_ended(&self) -> bool {
        Utc::now() > self.current_period_end
    }

    /// Activate the subscription (convert trial to active).
    pub fn activate(&mut self) -> Result<(), SaaSbillingError> {
        if !self.status.can_transition_to(&SubscriptionStatus::Active) {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: SubscriptionStatus::Active.to_string(),
            });
        }
        self.status = SubscriptionStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Cancel the subscription.
    pub fn cancel(&mut self, reason: Option<String>) -> Result<(), SaaSbillingError> {
        if self.status == SubscriptionStatus::Trialing {
            return Err(SaaSbillingError::CannotCancelDuringTrial);
        }
        if !self.status.can_transition_to(&SubscriptionStatus::Canceled) {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: SubscriptionStatus::Canceled.to_string(),
            });
        }
        self.status = SubscriptionStatus::Canceled;
        self.canceled_at = Some(Utc::now());
        self.cancel_reason = reason;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Pause the subscription.
    pub fn pause(&mut self) -> Result<(), SaaSbillingError> {
        if !self.status.can_transition_to(&SubscriptionStatus::Paused) {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: SubscriptionStatus::Paused.to_string(),
            });
        }
        self.status = SubscriptionStatus::Paused;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Resume the subscription.
    pub fn resume(&mut self) -> Result<(), SaaSbillingError> {
        if !self.status.can_transition_to(&SubscriptionStatus::Active) {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: SubscriptionStatus::Active.to_string(),
            });
        }
        self.status = SubscriptionStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark as past due.
    pub fn mark_past_due(&mut self) -> Result<(), SaaSbillingError> {
        if !self.status.can_transition_to(&SubscriptionStatus::PastDue) {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: SubscriptionStatus::PastDue.to_string(),
            });
        }
        self.status = SubscriptionStatus::PastDue;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Renew the subscription for the next period.
    pub fn renew(&mut self) -> Result<(), SaaSbillingError> {
        self.current_period_start = self.current_period_end;
        self.current_period_end = self.current_period_end + chrono::Duration::days(30);
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_subscription() -> TenantSubscription {
        TenantSubscription {
            subscription_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            plan_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            status: SubscriptionStatus::Active,
            current_period_start: chrono::Utc::now(),
            current_period_end: chrono::Utc::now() + chrono::Duration::days(30),
            trial_ends_at: None,
            canceled_at: None,
            cancel_reason: None,
            payment_method_id: None,
            stripe_subscription_id: None,
            created_by: Uuid::now_v7(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_subscription_is_active() {
        let mut sub = test_subscription();
        assert!(sub.is_active());

        sub.status = SubscriptionStatus::Canceled;
        assert!(!sub.is_active());

        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.is_active());
    }

    #[test]
    fn test_subscription_is_trialing() {
        let mut sub = test_subscription();
        assert!(!sub.is_trialing());

        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.is_trialing());
    }

    #[test]
    fn test_subscription_is_trial_expired() {
        let mut sub = test_subscription();
        sub.status = SubscriptionStatus::Trialing;
        sub.trial_ends_at = Some(chrono::Utc::now() - chrono::Duration::days(1));
        assert!(sub.is_trial_expired());

        sub.trial_ends_at = Some(chrono::Utc::now() + chrono::Duration::days(7));
        assert!(!sub.is_trial_expired());
    }

    #[test]
    fn test_subscription_status_allows_access() {
        assert!(SubscriptionStatus::Trialing.allows_access());
        assert!(SubscriptionStatus::Active.allows_access());
        assert!(SubscriptionStatus::PastDue.allows_access());
        assert!(!SubscriptionStatus::Canceled.allows_access());
        assert!(!SubscriptionStatus::Unpaid.allows_access());
        assert!(!SubscriptionStatus::Paused.allows_access());
    }

    #[test]
    fn test_subscription_status_transitions() {
        // Valid transitions
        assert!(SubscriptionStatus::Trialing.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::Trialing.can_transition_to(&SubscriptionStatus::Canceled));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::PastDue));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Canceled));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Paused));
        assert!(SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::Paused.can_transition_to(&SubscriptionStatus::Active));

        // Invalid transitions
        assert!(!SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Trialing.can_transition_to(&SubscriptionStatus::Paused));
    }

    #[test]
    fn test_subscription_activate() {
        let mut sub = test_subscription();
        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.activate().is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Active);
    }

    #[test]
    fn test_subscription_cancel() {
        let mut sub = test_subscription();
        assert!(sub.cancel(None).is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Canceled);
        assert!(sub.canceled_at.is_some());
    }

    #[test]
    fn test_subscription_cancel_during_trial_fails() {
        let mut sub = test_subscription();
        sub.status = SubscriptionStatus::Trialing;
        assert!(sub.cancel(None).is_err());
    }

    #[test]
    fn test_subscription_pause_and_resume() {
        let mut sub = test_subscription();
        assert!(sub.pause().is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Paused);
        assert!(sub.resume().is_ok());
        assert_eq!(sub.status, SubscriptionStatus::Active);
    }

    #[test]
    fn test_subscription_renew() {
        let mut sub = test_subscription();
        let old_start = sub.current_period_start;
        let old_end = sub.current_period_end;
        assert!(sub.renew().is_ok());
        assert_eq!(sub.current_period_start, old_end);
        assert_eq!(sub.current_period_end, old_end + chrono::Duration::days(30));
    }

    #[test]
    fn test_subscription_new_with_trial() {
        let sub = TenantSubscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Some(14),
        );
        assert_eq!(sub.status, SubscriptionStatus::Trialing);
        assert!(sub.trial_ends_at.is_some());
    }

    #[test]
    fn test_subscription_new_without_trial() {
        let sub = TenantSubscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
        );
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.trial_ends_at.is_none());
    }
}

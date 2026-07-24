//! Subscription Billing domain model — BC-08
//!
//! Event-sourced Subscription aggregate with renewal, dunning, pause/resume.
//!
//! File structure (one concept per file per CONVENTIONS.md):
//!
//! - [`error`]                — [`SubscriptionError`]
//! - [`plan`]                 — [`SubscriptionPlan`]
//! - [`billing_cycle`]        — [`BillingCycle`], [`BillingCycleStatus`]
//! - [`dunning`]              — [`DunningRetry`], [`DunningStatus`]
//! - [`subscription_status`]  — [`SubscriptionStatus`] state machine
//! - [`subscription`]         — [`Subscription`] aggregate root

pub mod billing_cycle;
pub mod dunning;
pub mod error;
pub mod plan;
pub mod subscription;
pub mod subscription_status;

pub use billing_cycle::*;
pub use dunning::*;
pub use error::*;
pub use plan::*;
pub use subscription::*;
pub use subscription_status::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_plan() -> SubscriptionPlan {
        SubscriptionPlan {
            plan_id: "plan_basic".into(),
            name: "Basic Monthly".into(),
            amount_minor_units: 999,
            currency: "USD".into(),
            billing_interval_days: 30,
            trial_period_days: None,
            is_active: true,
        }
    }

    #[test]
    fn test_subscription_status_valid_transitions() {
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::PastDue));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Cancelled));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Paused));
        assert!(SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Cancelled));
        assert!(SubscriptionStatus::Paused.can_transition_to(&SubscriptionStatus::Active));
    }

    #[test]
    fn test_subscription_status_invalid_transitions() {
        assert!(!SubscriptionStatus::Cancelled.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Paused.can_transition_to(&SubscriptionStatus::PastDue));
        assert!(!SubscriptionStatus::Cancelled.can_transition_to(&SubscriptionStatus::Paused));
        assert!(!SubscriptionStatus::PastDue.can_transition_to(&SubscriptionStatus::Paused));
    }

    #[test]
    fn test_subscription_status_from_str() {
        assert_eq!("active".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Active);
        assert_eq!("past_due".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::PastDue);
        assert_eq!("cancelled".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Cancelled);
        assert_eq!("paused".parse::<SubscriptionStatus>().unwrap(), SubscriptionStatus::Paused);
        assert!("invalid".parse::<SubscriptionStatus>().is_err());
    }

    #[test]
    fn test_subscription_status_display() {
        assert_eq!(format!("{}", SubscriptionStatus::Active), "active");
        assert_eq!(format!("{}", SubscriptionStatus::PastDue), "past_due");
        assert_eq!(format!("{}", SubscriptionStatus::Cancelled), "cancelled");
        assert_eq!(format!("{}", SubscriptionStatus::Paused), "paused");
    }

    #[test]
    fn test_subscription_new() {
        let plan = test_plan();
        let sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert_eq!(sub.plan_id, "plan_basic");
        assert_eq!(sub.plan_amount_minor_units, 999);
        assert_eq!(sub.currency, "USD");
        assert_eq!(sub.billing_interval_days, 30);
        assert_eq!(sub.dunning_retry_count, 0);
        assert_eq!(sub.max_dunning_retries, 3);
        assert_eq!(sub.billing_cycles.len(), 1);
        assert!(sub.dunning_retries.is_empty());
        assert!(sub.cancelled_at.is_none());
    }

    #[test]
    fn test_subscription_new_invalid_plan_rejected() {
        let plan = SubscriptionPlan {
            amount_minor_units: 0,
            ..test_plan()
        };
        let result = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(SubscriptionError::InvalidPlanAmount)));
    }

    #[test]
    fn test_subscription_cancel() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        sub.cancel(false).unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Cancelled);
        assert!(sub.cancelled_at.is_some());
    }

    #[test]
    fn test_subscription_cancel_twice_fails() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        sub.cancel(false).unwrap();
        let result = sub.cancel(false);
        assert!(result.is_err());
        assert!(matches!(result, Err(SubscriptionError::AlreadyCancelled)));
    }

    #[test]
    fn test_subscription_cancel_during_renewal_fails() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        let result = sub.cancel(true);
        assert!(result.is_err());
        assert!(matches!(result, Err(SubscriptionError::CannotCancelDuringRenewal)));
    }

    #[test]
    fn test_subscription_pause_and_resume() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        sub.pause().unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Paused);
        assert!(sub.paused_at.is_some());

        sub.resume().unwrap();
        assert_eq!(sub.status, SubscriptionStatus::Active);
        assert!(sub.resumed_at.is_some());
    }

    #[test]
    fn test_subscription_start_billing_cycle() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        let cycle = sub.start_billing_cycle().unwrap();
        assert_eq!(cycle.status, BillingCycleStatus::Billing);
        assert_eq!(sub.billing_cycles.len(), 2);
    }

    #[test]
    fn test_subscription_record_successful_payment() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        let cycle_id = sub.billing_cycles[0].billing_cycle_id;
        sub.record_successful_payment(cycle_id, Uuid::now_v7()).unwrap();
        assert_eq!(sub.billing_cycles[0].status, BillingCycleStatus::Succeeded);
    }

    #[test]
    fn test_subscription_failed_payment_triggers_dunning() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            3,
        )
        .unwrap();

        let cycle_id = sub.billing_cycles[0].billing_cycle_id;
        let retry = sub.record_failed_payment(cycle_id).unwrap();
        assert_eq!(retry.retry_number, 1);
        assert_eq!(sub.status, SubscriptionStatus::PastDue);
        assert_eq!(sub.dunning_retry_count, 1);
        // Retry is scheduled 1 day in the future — not immediately due
        assert!(!sub.is_dunning_due());
    }

    #[test]
    fn test_subscription_dunning_exhausted() {
        let plan = test_plan();
        let mut sub = Subscription::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &plan,
            None,
            1, // Only 1 retry allowed
        )
        .unwrap();

        let cycle_id = sub.billing_cycles[0].billing_cycle_id;
        let result = sub.record_failed_payment(cycle_id);
        assert!(result.is_err());
        assert!(matches!(result, Err(SubscriptionError::DunningExhausted)));
        assert_eq!(sub.status, SubscriptionStatus::Cancelled);
    }
}

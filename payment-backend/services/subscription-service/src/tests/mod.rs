//! Subscription Billing TDD tests — BC-08
//!
//! Spec test cases:
//! - test_create_subscription: basic creation
//! - test_subscription_renewal_idempotent: same cycle produces same key
//! - test_subscription_cannot_cancel_during_renewal: INV-SUB-01
//! - test_dunning_retry_schedule: failed payment → retry at day 1
//! - test_cancel_active_subscription: successful cancel
//! - test_pause_and_resume: lifecycle
//! - test_find_by_customer: query
//! - test_dunning_exhausted: max retries → cancelled

mod dunning_tests;
mod renewal_tests;
mod subscription_tests;
mod pg_repository_tests;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

pub(crate) fn make_plan() -> SubscriptionPlan {
    SubscriptionPlan {
        plan_id: "plan_monthly_001".into(),
        name: "Monthly Premium".into(),
        amount_minor_units: 9999,
        currency: "AED".into(),
        billing_interval_days: 30,
        trial_period_days: None,
        is_active: true,
    }
}

pub(crate) fn setup() -> SubscriptionPipeline {
    SubscriptionPipeline::new()
}

pub(crate) async fn create_test_subscription(pipeline: &SubscriptionPipeline) -> Subscription {
    let cmd = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id: Uuid::now_v7(),
        plan: make_plan(),
        payment_method_token_id: Some(Uuid::now_v7()),
        max_dunning_retries: Some(3),
    };
    pipeline.api.create_subscription(cmd).await.unwrap()
}

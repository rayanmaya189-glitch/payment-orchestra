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

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_plan() -> SubscriptionPlan {
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

fn setup() -> SubscriptionPipeline {
    SubscriptionPipeline::new()
}

async fn create_test_subscription(pipeline: &SubscriptionPipeline) -> Subscription {
    let cmd = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id: Uuid::now_v7(),
        plan: make_plan(),
        payment_method_token_id: Some(Uuid::now_v7()),
        max_dunning_retries: Some(3),
    };
    pipeline.api.create_subscription(cmd).await.unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_subscription() {
    let pipeline = setup();
    let cmd = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id: Uuid::now_v7(),
        plan: make_plan(),
        payment_method_token_id: Some(Uuid::now_v7()),
        max_dunning_retries: Some(3),
    };

    let sub = pipeline.api.create_subscription(cmd).await.unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.plan_amount_minor_units, 9999);
    assert_eq!(sub.currency, "AED");
    assert_eq!(sub.max_dunning_retries, 3);
    assert_eq!(sub.billing_cycles.len(), 1);
    assert!(sub.current_period_end > sub.current_period_start);
}

#[tokio::test]
async fn test_subscription_renewal_idempotent() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    // First renewal creates a billing cycle
    let r1 = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    // Second renewal should return the same billing cycle (idempotent)
    let r2 = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    assert_eq!(
        r1.billing_cycle_id, r2.billing_cycle_id,
        "Renewal should be idempotent"
    );
    assert_eq!(
        r1.idempotency_key, r2.idempotency_key,
        "Idempotency key should match"
    );
    assert_eq!(r1.amount_minor_units, 9999);
}

#[tokio::test]
async fn test_subscription_cannot_cancel_during_renewal() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    // Start a renewal (simulates saga in progress)
    let _renewal = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    // Try to cancel with has_running_renewal = true
    let result = pipeline
        .api
        .cancel_subscription(CancelSubscriptionCommand {
            subscription_id: sub.subscription_id,
            reason: None,
            has_running_renewal: true,
        })
        .await;

    assert!(result.is_err(), "Cancelling during renewal should fail");
}

#[tokio::test]
async fn test_cancel_active_subscription() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    let cancelled = pipeline
        .api
        .cancel_subscription(CancelSubscriptionCommand {
            subscription_id: sub.subscription_id,
            reason: Some("Customer requested".into()),
            has_running_renewal: false,
        })
        .await
        .unwrap();

    assert_eq!(cancelled.status, SubscriptionStatus::Cancelled);
    assert!(cancelled.cancelled_at.is_some());
}

#[tokio::test]
async fn test_cancel_already_cancelled_rejected() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    // Cancel once
    pipeline
        .api
        .cancel_subscription(CancelSubscriptionCommand {
            subscription_id: sub.subscription_id,
            reason: None,
            has_running_renewal: false,
        })
        .await
        .unwrap();

    // Cancel again should fail
    let result = pipeline
        .api
        .cancel_subscription(CancelSubscriptionCommand {
            subscription_id: sub.subscription_id,
            reason: None,
            has_running_renewal: false,
        })
        .await;
    assert!(result.is_err(), "Double cancel should fail");
}

#[tokio::test]
async fn test_pause_and_resume_subscription() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    // Pause
    let paused = pipeline
        .api
        .pause_subscription(PauseSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();
    assert_eq!(paused.status, SubscriptionStatus::Paused);

    // Resume
    let resumed = pipeline
        .api
        .resume_subscription(ResumeSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();
    assert_eq!(resumed.status, SubscriptionStatus::Active);
}

#[tokio::test]
async fn test_dunning_retry_schedule() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    // Start renewal
    let renewal = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    // Fail the payment
    let failed = pipeline
        .api
        .fail_renewal_payment(FailRenewalPaymentCommand {
            subscription_id: sub.subscription_id,
            billing_cycle_id: renewal.billing_cycle_id,
            reason: "Insufficient funds".into(),
        })
        .await
        .unwrap();

    assert_eq!(failed.status, SubscriptionStatus::PastDue);
    assert_eq!(failed.dunning_retry_count, 1);
    assert_eq!(failed.dunning_retries.len(), 1);
    assert_eq!(failed.dunning_retries[0].status, DunningStatus::Pending);
}

#[tokio::test]
async fn test_dunning_exhausted_transitions_to_cancelled() {
    let pipeline = setup();

    let cmd = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id: Uuid::now_v7(),
        plan: make_plan(),
        payment_method_token_id: Some(Uuid::now_v7()),
        max_dunning_retries: Some(1), // Only 1 retry allowed
    };
    let sub = pipeline.api.create_subscription(cmd).await.unwrap();

    // Start renewal
    let renewal = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    // First failure with max_dunning_retries = 1 exhausts and cancels
    let failed = pipeline
        .api
        .fail_renewal_payment(FailRenewalPaymentCommand {
            subscription_id: sub.subscription_id,
            billing_cycle_id: renewal.billing_cycle_id,
            reason: "Insufficient funds".into(),
        })
        .await
        .unwrap();

    assert_eq!(
        failed.status,
        SubscriptionStatus::Cancelled,
        "Dunning exhausted should cancel subscription"
    );
    assert!(failed.cancelled_at.is_some());
}

#[tokio::test]
async fn test_confirm_renewal_payment() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    let renewal = pipeline
        .api
        .renew_subscription(RenewSubscriptionCommand {
            subscription_id: sub.subscription_id,
        })
        .await
        .unwrap();

    let confirmed = pipeline
        .api
        .confirm_renewal_payment(ConfirmRenewalPaymentCommand {
            subscription_id: sub.subscription_id,
            billing_cycle_id: renewal.billing_cycle_id,
            payment_intent_id: Uuid::now_v7(),
        })
        .await
        .unwrap();

    assert_eq!(confirmed.status, SubscriptionStatus::Active);
}

#[tokio::test]
async fn test_find_by_customer() {
    let pipeline = setup();
    let customer_id = Uuid::now_v7();

    // Create two subscriptions for the same customer
    let cmd1 = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id,
        plan: make_plan(),
        payment_method_token_id: None,
        max_dunning_retries: Some(3),
    };
    pipeline.api.create_subscription(cmd1).await.unwrap();

    let cmd2 = CreateSubscriptionCommand {
        operator_id: Uuid::now_v7(),
        customer_id,
        plan: SubscriptionPlan {
            plan_id: "plan_yearly_002".into(),
            name: "Yearly Premium".into(),
            amount_minor_units: 99900,
            currency: "AED".into(),
            billing_interval_days: 365,
            trial_period_days: Some(7),
            is_active: true,
        },
        payment_method_token_id: None,
        max_dunning_retries: Some(3),
    };
    pipeline.api.create_subscription(cmd2).await.unwrap();

    let results = pipeline.api.find_by_customer(customer_id).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_query_subscription_by_id() {
    let pipeline = setup();
    let sub = create_test_subscription(&pipeline).await;

    let fetched = pipeline.api.get_subscription(sub.subscription_id).await.unwrap();
    assert_eq!(fetched.subscription_id, sub.subscription_id);
    assert_eq!(fetched.plan_id, "plan_monthly_001");
}

#[tokio::test]
async fn test_query_nonexistent_subscription() {
    let pipeline = setup();
    let result = pipeline.api.get_subscription(Uuid::now_v7()).await;
    assert!(result.is_err(), "Nonexistent subscription should error");
}

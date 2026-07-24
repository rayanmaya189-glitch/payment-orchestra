//! Subscription tests: create, cancel, pause/resume.

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{make_plan, setup, create_test_subscription};

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

//! Dunning tests: dunning retry schedule, exhausted transitions.

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{make_plan, setup, create_test_subscription};

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

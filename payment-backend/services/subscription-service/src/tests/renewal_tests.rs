//! Renewal tests: renewal, confirm/fail payment.

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{make_plan, setup, create_test_subscription};

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

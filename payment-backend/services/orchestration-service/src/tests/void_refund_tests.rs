//! Void and Refund PaymentIntent tests.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;

use super::{setup_handler, setup_authorized_intent};

#[tokio::test]
async fn test_void_authorized_intent() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    let result = handler.void_payment_intent(VoidPaymentIntent {
        payment_intent_id: pi_id,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.status, PaymentStatus::Voided);
}

#[tokio::test]
async fn test_void_after_capture_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    // Capture first
    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    // Now try to void
    let result = handler.void_payment_intent(VoidPaymentIntent {
        payment_intent_id: pi_id,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_void_already_voided_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    // Void
    handler.void_payment_intent(VoidPaymentIntent {
        payment_intent_id: pi_id,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    // Try to void again
    let result = handler.void_payment_intent(VoidPaymentIntent {
        payment_intent_id: pi_id,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_refund_full_amount() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    // Capture
    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    // Refund
    let result = handler.refund_payment_intent(RefundPaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: 10000,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.status, PaymentStatus::Refunded);
    assert_eq!(result.refunded_amount.amount_minor_units, 10000);
}

#[tokio::test]
async fn test_refund_partial_amount() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    let result = handler.refund_payment_intent(RefundPaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: 3000,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.status, PaymentStatus::PartiallyRefunded);
    assert_eq!(result.refunded_amount.amount_minor_units, 3000);
}

#[tokio::test]
async fn test_refund_exceeds_balance_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    let result = handler.refund_payment_intent(RefundPaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: 20000, // exceeds captured 10000
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_refund_zero_amount_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    let result = handler.refund_payment_intent(RefundPaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: 0,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_refund_on_voided_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    handler.void_payment_intent(VoidPaymentIntent {
        payment_intent_id: pi_id,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    let result = handler.refund_payment_intent(RefundPaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: 1000,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

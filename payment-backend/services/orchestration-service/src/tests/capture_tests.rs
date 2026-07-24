//! Capture PaymentIntent tests: full capture, partial, exceeds, rejected.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;
use crate::repository::*;

use super::{make_create_cmd, make_activate_policy_cmd, make_authorize_cmd, setup_handler, setup_authorized_intent};

#[tokio::test]
async fn test_capture_full_amount() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: None, // full capture
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.status, PaymentStatus::Captured);
    assert_eq!(result.captured_amount.amount_minor_units, 10000);
}

#[tokio::test]
async fn test_capture_partial_amount() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(3000),
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.status, PaymentStatus::PartiallyCaptured);
    assert_eq!(result.captured_amount.amount_minor_units, 3000);
}

#[tokio::test]
async fn test_capture_exceeds_authorized_rejected() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(20000), // exceeds authorized 10000
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_capture_partial_not_supported() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(3000),
        supports_partial_capture: false, // connector doesn't support partial
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_capture_on_failed_intent_rejected() {
    let (handler, _repo) = setup_handler();
    let operator_id = Uuid::now_v7();

    // Create but never authorize
    let pi = handler.create_payment_intent(make_create_cmd(operator_id, "cap-fail", 10000)).await.unwrap();

    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi.payment_intent_id,
        amount_minor_units: None,
        supports_partial_capture: true,
        max_partial_captures: 3,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err()); // PAYMENT_INTENT_NOT_AUTHORIZED
}

#[tokio::test]
async fn test_inv_01_partial_captures_sum_never_exceeds_authorized() {
    let (handler, repo) = setup_handler();
    let operator_id = Uuid::now_v7();
    let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

    // Capture 3000
    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(3000),
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    // Capture 4000
    handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(4000),
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await.unwrap();

    // Capture 4000 → would exceed 10000 (total would be 11000)
    let result = handler.capture_payment_intent(CapturePaymentIntent {
        payment_intent_id: pi_id,
        amount_minor_units: Some(4000),
        supports_partial_capture: true,
        max_partial_captures: 10,
        actor_id: Uuid::now_v7(),
    }).await;

    assert!(result.is_err(), "Should reject capture that exceeds authorized amount");
}

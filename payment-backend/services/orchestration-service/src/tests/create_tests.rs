//! Create PaymentIntent tests.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;

use super::{make_create_cmd, setup_handler};

#[tokio::test]
async fn test_create_payment_intent_success() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();

    let result = handler.create_payment_intent(make_create_cmd(operator_id, "key-001", 10000)).await.unwrap();

    assert_eq!(result.status, PaymentStatus::Created);
    assert_eq!(result.requested_amount.amount_minor_units, 10000);
    assert_eq!(result.events.len(), 1);
}

#[tokio::test]
async fn test_create_payment_intent_idempotent_replay() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();

    let cmd = make_create_cmd(operator_id, "key-002", 5000);
    let r1 = handler.create_payment_intent(cmd.clone()).await.unwrap();
    let r2 = handler.create_payment_intent(cmd).await.unwrap();

    assert_eq!(r1.payment_intent_id, r2.payment_intent_id);
    assert_eq!(r1.status, r2.status);
    assert_eq!(r1.requested_amount.amount_minor_units, r2.requested_amount.amount_minor_units);
}

#[tokio::test]
async fn test_create_payment_intent_zero_amount_card_verification() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();

    let mut cmd = make_create_cmd(operator_id, "key-003", 0);
    cmd.purpose = PaymentPurpose::CardVerification;

    let result = handler.create_payment_intent(cmd).await.unwrap();
    assert!(result.requested_amount.amount_minor_units == 0);
}

#[tokio::test]
async fn test_create_payment_intent_invalid_currency() {
    let (handler, _) = setup_handler();
    let operator_id = Uuid::now_v7();

    let mut cmd = make_create_cmd(operator_id, "key-004", 10000);
    cmd.currency = "INVALID".to_string();

    let result = handler.create_payment_intent(cmd).await;
    assert!(result.is_err());
}

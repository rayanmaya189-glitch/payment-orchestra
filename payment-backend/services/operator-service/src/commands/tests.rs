//! Tests for Operator Management command handlers.

use super::*;
use uuid::Uuid;
use crate::repository::InMemoryOperatorRepository;
use crate::domain::OperatorStatus;

fn create_handler() -> OperatorCommandHandler<InMemoryOperatorRepository> {
    OperatorCommandHandler::new(InMemoryOperatorRepository::new())
}

#[tokio::test]
async fn test_register_operator_success() {
    let handler = create_handler();
    let result = handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    assert_eq!(result.operator.status, OperatorStatus::Pending);
    assert!(!result.verification_token.is_empty());
    assert!(result.operator.verification_token_hash.is_some());
}

#[tokio::test]
async fn test_register_duplicate_trade_license_rejected() {
    let handler = create_handler();
    handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    let result = handler.register(RegisterOperator {
        legal_name: "Acme Corp 2".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin2@acme.com".into(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_email_verification_transitions() {
    let handler = create_handler();
    let registered = handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    let result = handler.verify_email(VerifyEmail {
        operator_id: registered.operator.id,
        verification_token: registered.verification_token,
    }).await.unwrap();

    assert_eq!(result.operator.status, OperatorStatus::ActiveUnverified);
}

#[tokio::test]
async fn test_email_verification_wrong_token_rejected() {
    let handler = create_handler();
    let registered = handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    let result = handler.verify_email(VerifyEmail {
        operator_id: registered.operator.id,
        verification_token: "wrong-token".into(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_nonexistent_operator_fails() {
    let handler = create_handler();
    let result = handler.verify_email(VerifyEmail {
        operator_id: Uuid::now_v7(),
        verification_token: "token".into(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_status_verified() {
    let handler = create_handler();
    let registered = handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    handler.verify_email(VerifyEmail {
        operator_id: registered.operator.id,
        verification_token: registered.verification_token,
    }).await.unwrap();

    let result = handler.update_status(UpdateOperatorStatus {
        operator_id: registered.operator.id,
        new_status: OperatorStatus::ActiveVerified,
        reason: "KYB approved".into(),
        changed_by: Uuid::now_v7(),
    }).await.unwrap();

    assert_eq!(result.operator.status, OperatorStatus::ActiveVerified);
}

#[tokio::test]
async fn test_invalid_status_transition_rejected() {
    let handler = create_handler();
    let registered = handler.register(RegisterOperator {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();

    let result = handler.update_status(UpdateOperatorStatus {
        operator_id: registered.operator.id,
        new_status: OperatorStatus::ActiveVerified,
        reason: "Should not work".into(),
        changed_by: Uuid::now_v7(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_trade_license_format() {
    let handler = create_handler();
    let result = handler.register(RegisterOperator {
        legal_name: "Test".into(),
        trade_license_no: "AB".into(),
        country: "AE".into(),
        email: "test@test.com".into(),
    }).await;

    assert!(result.is_err());
}

//! Tests for IAM command handlers.

use super::*;
use uuid::Uuid;
use crate::domain::{ApiKeyStatus, ChangeStatus, Principal};
use crate::repository::{InMemoryIamRepository, IamRepository};

fn create_handler() -> IamCommandHandler<InMemoryIamRepository> {
    IamCommandHandler::new(InMemoryIamRepository::new(), "test-secret".into())
}

fn create_test_principal(repo: &InMemoryIamRepository) -> Principal {
    let principal = Principal::new_human(
        Uuid::now_v7(),
        "admin@test.com".into(),
        ring::digest::digest(&ring::digest::SHA256, b"password123").as_ref().to_vec(),
    );
    let _ = futures::executor::block_on(repo.save_principal(&principal));
    principal
}

#[tokio::test]
async fn test_authenticate_success() {
    let repo = InMemoryIamRepository::new();
    create_test_principal(&repo);
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    let result = handler.authenticate(Authenticate {
        email: "admin@test.com".into(),
        password: "password123".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        user_agent: "test-agent".into(),
    }).await.unwrap();

    assert!(!result.access_token.is_empty());
    assert!(!result.refresh_token.is_empty());
}

#[tokio::test]
async fn test_authenticate_wrong_password() {
    let repo = InMemoryIamRepository::new();
    create_test_principal(&repo);
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    let result = handler.authenticate(Authenticate {
        email: "admin@test.com".into(),
        password: "wrong-password".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        user_agent: "test-agent".into(),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_api_key_success() {
    let repo = InMemoryIamRepository::new();
    let principal = create_test_principal(&repo);
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    let result = handler.create_api_key(CreateApiKey {
        principal_id: principal.id,
        name: "Production Key".into(),
        scopes: vec!["payments:read".into(), "payments:write".into()],
        expires_in_days: Some(90),
    }).await.unwrap();

    assert_eq!(result.api_key.status, ApiKeyStatus::Active);
    assert!(!result.api_key_secret.is_empty());
}

#[tokio::test]
async fn test_create_duplicate_api_key_rejected() {
    let repo = InMemoryIamRepository::new();
    let principal = create_test_principal(&repo);
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    handler.create_api_key(CreateApiKey {
        principal_id: principal.id,
        name: "Production Key".into(),
        scopes: vec!["payments:read".into()],
        expires_in_days: Some(90),
    }).await.unwrap();

    let result = handler.create_api_key(CreateApiKey {
        principal_id: principal.id,
        name: "Production Key".into(),
        scopes: vec!["payments:read".into()],
        expires_in_days: Some(90),
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_revoke_api_key() {
    let repo = InMemoryIamRepository::new();
    let principal = create_test_principal(&repo);
    let handler = IamCommandHandler::new(repo.clone(), "test-secret".into());

    let created = handler.create_api_key(CreateApiKey {
        principal_id: principal.id,
        name: "Key to Revoke".into(),
        scopes: vec!["payments:read".into()],
        expires_in_days: Some(90),
    }).await.unwrap();

    let result = handler.revoke_api_key(RevokeApiKey {
        api_key_id: created.api_key.api_key_id,
        principal_id: principal.id,
    }).await.unwrap();

    assert!(result.revoked);
}

#[tokio::test]
async fn test_submit_and_approve_change() {
    let repo = InMemoryIamRepository::new();
    let maker_id = Uuid::now_v7();
    let checker_id = Uuid::now_v7();
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    let submitted = handler.submit_change(SubmitChange {
        change_type: "update_gateway_config".into(),
        maker_id,
        payload: vec![1, 2, 3],
        maker_note: Some("Update fee structure".into()),
    }).await.unwrap();

    assert_eq!(submitted.change.status, ChangeStatus::Pending);

    let reviewed = handler.review_change(ReviewChange {
        change_id: submitted.change.change_id,
        checker_id,
        approved: true,
        checker_note: Some("Approved".into()),
    }).await.unwrap();

    assert_eq!(reviewed.change.status, ChangeStatus::Approved);
}

#[tokio::test]
async fn test_self_approval_rejected() {
    let repo = InMemoryIamRepository::new();
    let maker_id = Uuid::now_v7();
    let handler = IamCommandHandler::new(repo, "test-secret".into());

    let submitted = handler.submit_change(SubmitChange {
        change_type: "test".into(),
        maker_id,
        payload: vec![],
        maker_note: None,
    }).await.unwrap();

    let result = handler.review_change(ReviewChange {
        change_id: submitted.change.change_id,
        checker_id: maker_id,
        approved: true,
        checker_note: None,
    }).await;

    assert!(result.is_err());
}

//! Tests for MerchantAcquirerLink command handlers.

use super::*;
use uuid::Uuid;
use crate::repository::InMemoryLinkRepository;
use crate::domain::{LinkStatus, LinkEnvironment, MerchantAcquirerLink};

async fn setup() -> LinkCommandHandler<InMemoryLinkRepository> {
    let repo = InMemoryLinkRepository::new();
    LinkCommandHandler::new(repo)
}

#[tokio::test]
async fn test_create_link_success() {
    let handler = setup().await;
    let mut creds = std::collections::HashMap::new();
    creds.insert("secret_key".into(), "sk_test_abc".into());

    let result = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "checkout_com".into(),
        display_name: "Production Gateway".into(),
        environment: LinkEnvironment::Production,
        credentials: creds,
    }).await.unwrap();

    assert_eq!(result.link.status, LinkStatus::Testing);
    assert_eq!(result.link.connector_id, "checkout_com");
}

#[tokio::test]
async fn test_create_link_invalid_connector() {
    let handler = setup().await;
    let creds = std::collections::HashMap::new();

    let result = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "".into(),
        display_name: "Test".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: creds,
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_then_test_connection() {
    let handler = setup().await;
    let mut creds = std::collections::HashMap::new();
    creds.insert("api_key".into(), "sk_test_xyz".into());

    let created = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "network_international".into(),
        display_name: "NI Sandbox".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: creds,
    }).await.unwrap();

    assert_eq!(created.link.status, LinkStatus::Testing);

    let tested = handler.test_connection(TestConnection {
        link_id: created.link.link_id,
    }).await.unwrap();

    assert!(tested.success);
    assert_eq!(tested.link.status, LinkStatus::Active);
}

#[tokio::test]
async fn test_disable_and_enable() {
    let handler = setup().await;
    let creds = std::collections::HashMap::new();

    let created = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "telr".into(),
        display_name: "Telr Test".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: creds,
    }).await.unwrap();

    handler.disable_link(DisableLink {
        link_id: created.link.link_id,
        reason: "maintenance".into(),
    }).await.unwrap();

    let enabled = handler.enable_link(EnableLink {
        link_id: created.link.link_id,
    }).await.unwrap();

    assert_eq!(enabled.link.status, LinkStatus::Testing);
}

#[tokio::test]
async fn test_rotate_credentials() {
    let handler = setup().await;
    let mut old_creds = std::collections::HashMap::new();
    old_creds.insert("key".into(), "old_value".into());

    let created = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "checkout_com".into(),
        display_name: "Rotate Test".into(),
        environment: LinkEnvironment::Production,
        credentials: old_creds,
    }).await.unwrap();

    let mut new_creds = std::collections::HashMap::new();
    new_creds.insert("key".into(), "new_value".into());

    let rotated = handler.rotate_credentials(RotateCredentials {
        link_id: created.link.link_id,
        new_credentials: new_creds,
        rotate_immediately: true,
    }).await.unwrap();

    let expected_hash = {
        let mut m = std::collections::HashMap::<String, String>::new();
        m.insert("key".to_string(), "new_value".to_string());
        MerchantAcquirerLink::compute_credentials_hash(&serde_json::to_string(&m).unwrap())
    };
    assert_eq!(rotated.link.credentials_hash, expected_hash);
}

#[tokio::test]
async fn test_update_metadata() {
    let handler = setup().await;
    let creds = std::collections::HashMap::new();

    let created = handler.create_link(CreateLink {
        operator_id: Uuid::now_v7(),
        connector_id: "checkout_com".into(),
        display_name: "Old Name".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: creds,
    }).await.unwrap();

    let updated = handler.update_metadata(UpdateMetadata {
        link_id: created.link.link_id,
        display_name: Some("New Name".into()),
    }).await.unwrap();

    assert_eq!(updated.link.display_name, "New Name");
}

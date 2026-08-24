//! Integration tests for merchant-acquirer-link service.

mod pg_repository_tests;

use uuid::Uuid;
use crate::commands::{
    LinkCommandHandler, CreateLink, TestConnection, RotateCredentials,
    DisableLink, EnableLink, UpdateMetadata, CommandHandler,
};
use crate::queries::{LinkQueries, QueryHandler};
use crate::domain::{LinkEnvironment, LinkStatus, HealthStatus};
use crate::repository::InMemoryLinkRepository;
use std::collections::HashMap;

fn test_credentials() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("secret_key".into(), "sk_test_abc123".into());
    m.insert("public_key".into(), "pk_test_xyz789".into());
    m
}

async fn setup() -> (LinkCommandHandler<InMemoryLinkRepository>, LinkQueries<InMemoryLinkRepository>) {
    let repo = InMemoryLinkRepository::new();
    let handler = LinkCommandHandler::new(repo.clone());
    let queries = LinkQueries::new(repo);
    (handler, queries)
}

#[tokio::test]
async fn test_full_link_lifecycle() {
    let (handler, queries) = setup().await;
    let operator_id = Uuid::now_v7();

    // 1. Create link
    let created = handler.create_link(CreateLink {
        operator_id,
        connector_id: "checkout_com".into(),
        display_name: "Production Gateway".into(),
        environment: LinkEnvironment::Production,
        credentials: test_credentials(),
    }).await.unwrap();

    assert_eq!(created.link.status, LinkStatus::Testing);
    assert_eq!(created.link.health_status, HealthStatus::Unknown);

    // 2. Test connection
    let tested = handler.test_connection(TestConnection {
        link_id: created.link.link_id,
    }).await.unwrap();

    assert!(tested.success);
    assert_eq!(tested.link.status, LinkStatus::Active);
    assert_eq!(tested.link.health_status, HealthStatus::Healthy);

    // 3. Disable
    handler.disable_link(DisableLink {
        link_id: created.link.link_id,
        reason: "maintenance".into(),
    }).await.unwrap();

    let link = queries.get_link(created.link.link_id).await.unwrap().unwrap();
    assert_eq!(link.status, LinkStatus::Disabled);

    // 4. Re-enable
    handler.enable_link(EnableLink {
        link_id: created.link.link_id,
    }).await.unwrap();

    let link = queries.get_link(created.link.link_id).await.unwrap().unwrap();
    assert_eq!(link.status, LinkStatus::Testing);

    // 5. List links for operator
    let links = queries.list_links(operator_id, None).await.unwrap();
    assert_eq!(links.len(), 1);
}

#[tokio::test]
async fn test_credential_rotation() {
    let (handler, _) = setup().await;
    let operator_id = Uuid::now_v7();

    let created = handler.create_link(CreateLink {
        operator_id,
        connector_id: "network_international".into(),
        display_name: "NI Gateway".into(),
        environment: LinkEnvironment::Production,
        credentials: test_credentials(),
    }).await.unwrap();

    let mut new_creds = HashMap::new();
    new_creds.insert("api_key".into(), "new_key_value".into());

    let rotated = handler.rotate_credentials(RotateCredentials {
        link_id: created.link.link_id,
        new_credentials: new_creds,
        rotate_immediately: false,
    }).await.unwrap();

    // After rotation, status resets to testing (needs re-test)
    assert_eq!(rotated.link.status, LinkStatus::Testing);
    assert!(rotated.old_credentials_retained);
}

#[tokio::test]
async fn test_update_metadata() {
    let (handler, _) = setup().await;
    let operator_id = Uuid::now_v7();

    let created = handler.create_link(CreateLink {
        operator_id,
        connector_id: "telr".into(),
        display_name: "Old Name".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: test_credentials(),
    }).await.unwrap();

    let updated = handler.update_metadata(UpdateMetadata {
        link_id: created.link.link_id,
        display_name: Some("New Custom Name".into()),
    }).await.unwrap();

    assert_eq!(updated.link.display_name, "New Custom Name");
}

#[tokio::test]
async fn test_max_links_per_connector() {
    let (handler, _) = setup().await;
    let operator_id = Uuid::now_v7();

    // Create 5 links (max per connector)
    for i in 0..5 {
        let mut creds = HashMap::new();
        creds.insert("key".into(), format!("value_{}", i));

        // First make each link active by testing it
        let created = handler.create_link(CreateLink {
            operator_id,
            connector_id: "checkout_com".into(),
            display_name: format!("Gateway {}", i),
            environment: LinkEnvironment::Sandbox,
            credentials: creds,
        }).await.unwrap();

        handler.test_connection(TestConnection {
            link_id: created.link.link_id,
        }).await.unwrap();
    }

    // 6th should fail
    let mut creds = HashMap::new();
    creds.insert("key".into(), "value_6".into());
    let result = handler.create_link(CreateLink {
        operator_id,
        connector_id: "checkout_com".into(),
        display_name: "6th Gateway".into(),
        environment: LinkEnvironment::Sandbox,
        credentials: creds,
    }).await;

    assert!(result.is_err());
}

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use std::collections::HashMap;
use uuid::Uuid;

fn setup() -> OnboardingPipeline { OnboardingPipeline::new() }

async fn create_link(pipeline: &OnboardingPipeline) -> OnboardingRequest {
    let cmd = InitiateOnboardingCommand {
        operator_id: Uuid::now_v7(),
        connector_id: "network_international".into(),
        display_name: "NI Production".into(),
        environment: "sandbox".into(),
    };
    pipeline.api.initiate_onboarding(cmd).await.unwrap()
}

fn valid_creds() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("merchant_id".into(), "MER-12345".into());
    m.insert("api_key".into(), "ni_live_abc123def4567890abc123def45678".into());
    m.insert("environment".into(), "sandbox".into());
    m
}

fn short_creds() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("merchant_id".into(), "MER-12345".into());
    m.insert("api_key".into(), "ni_live_abc123def4567890abc123def45678".into());
    m.insert("environment".into(), "sandbox".into());
    m
}

#[tokio::test]
async fn test_initiate_onboarding() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;
    assert_eq!(req.status, OnboardingStatus::Draft);
    assert_eq!(req.connector_id, "network_international");
    assert_eq!(req.environment, "sandbox");
}

#[tokio::test]
async fn test_initiate_invalid_connector_rejected() {
    let pipeline = setup();
    let cmd = InitiateOnboardingCommand {
        operator_id: Uuid::now_v7(),
        connector_id: "nonexistent".into(),
        display_name: "Test".into(),
        environment: "sandbox".into(),
    };
    let result = pipeline.api.initiate_onboarding(cmd).await;
    assert!(result.is_err(), "Invalid connector should be rejected");
}

#[tokio::test]
async fn test_submit_credentials() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;
    let updated = pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: short_creds(),
    }).await.unwrap();

    assert_eq!(updated.status, OnboardingStatus::CredentialsSubmitted);
}

#[tokio::test]
async fn test_submit_credentials_missing_required_rejected() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;
    let mut creds = HashMap::new();
    creds.insert("merchant_id".into(), "MERCH12345".into());
    // Missing api_key (required)

    let result = pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: creds,
    }).await;
    assert!(result.is_err(), "Missing required field should be rejected");
}

#[tokio::test]
async fn test_test_connection() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;

    pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: short_creds(),
    }).await.unwrap();

    let tested = pipeline.api.test_connection(TestConnectionCommand {
        link_id: req.link_id,
    }).await.unwrap();

    assert_eq!(tested.status, OnboardingStatus::Active);
    assert_eq!(tested.health_status, HealthStatus::Healthy);
    assert!(tested.last_test_result.unwrap().success);
}

#[tokio::test]
async fn test_complete_test_success() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;

    pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: valid_creds(),
    }).await.unwrap();

    let result = pipeline.api.complete_test(CompleteTestCommand {
        link_id: req.link_id,
        result: ConnectionTestResult {
            success: true,
            latency_ms: 150,
            error: None,
            merchant_name: Some("Test".into()),
            permissions: vec!["authorize".into()],
        },
    }).await.unwrap();

    assert_eq!(result.status, OnboardingStatus::Active);
}

#[tokio::test]
async fn test_deactivate_and_reactivate() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;

    // Deactivate draft → should fail (invalid transition)
    let deact = pipeline.api.deactivate_link(DeactivateLinkCommand {
        link_id: req.link_id,
        reason: None,
    }).await;
    assert!(deact.is_err(), "Draft → Deactivated should fail");

    // Submit creds, test to activate
    pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: valid_creds(),
    }).await.unwrap();
    pipeline.api.test_connection(TestConnectionCommand { link_id: req.link_id }).await.unwrap();

    // Now deactivate
    let deactivated = pipeline.api.deactivate_link(DeactivateLinkCommand {
        link_id: req.link_id,
        reason: Some("Maintenance".into()),
    }).await.unwrap();
    assert_eq!(deactivated.status, OnboardingStatus::Deactivated);
}

#[tokio::test]
async fn test_revoke_link() {
    let pipeline = setup();
    let req = create_link(&pipeline).await;

    pipeline.api.submit_credentials(SubmitCredentialsCommand {
        link_id: req.link_id,
        credentials: valid_creds(),
    }).await.unwrap();
    pipeline.api.test_connection(TestConnectionCommand { link_id: req.link_id }).await.unwrap();

    let revoked = pipeline.api.revoke_link(RevokeLinkCommand {
        link_id: req.link_id,
        reason: "Security policy".into(),
    }).await.unwrap();
    assert_eq!(revoked.status, OnboardingStatus::Revoked);
}

#[tokio::test]
async fn test_list_connectors() {
    let pipeline = setup();
    let connectors = pipeline.api.list_connectors().await;
    assert_eq!(connectors.len(), 2);
    assert!(connectors.iter().any(|c| c.connector_id == "network_international"));
    assert!(connectors.iter().any(|c| c.connector_id == "checkout_com"));
}

#[tokio::test]
async fn test_get_connector_schema() {
    let pipeline = setup();
    let schema = pipeline.api.get_connector_schema("checkout_com").await.unwrap();
    assert!(schema.fields.iter().any(|f| f.name == "secret_key"));
    assert!(schema.fields.iter().any(|f| f.name == "public_key"));
}

#[tokio::test]
async fn test_nonexistent_onboarding() {
    let pipeline = setup();
    let result = pipeline.api.get_onboarding(Uuid::now_v7()).await;
    assert!(result.is_err());
}

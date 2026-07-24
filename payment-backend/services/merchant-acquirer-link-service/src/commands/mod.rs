//! Command handlers for BYOK Core — MerchantAcquirerLink lifecycle management.

pub mod types;
pub(crate) mod link;
pub(crate) mod credential;

pub use types::*;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::domain::LinkEvent;
use crate::repository::LinkRepository;

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_link(&self, cmd: CreateLink) -> Result<CreateLinkResult, crate::domain::LinkError>;
    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, crate::domain::LinkError>;
    async fn rotate_credentials(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, crate::domain::LinkError>;
    async fn disable_link(&self, cmd: DisableLink) -> Result<DisableLinkResult, crate::domain::LinkError>;
    async fn enable_link(&self, cmd: EnableLink) -> Result<EnableLinkResult, crate::domain::LinkError>;
    async fn update_metadata(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, crate::domain::LinkError>;
}

// ─── Command Handler ────────────────────────────────────────────────────────

pub struct LinkCommandHandler<R: LinkRepository> {
    pub(crate) repository: R,
    pub(crate) event_bus: Option<Arc<dyn EventBus>>,
}

impl<R: LinkRepository> LinkCommandHandler<R> {
    pub fn new(repository: R) -> Self {
        Self { repository, event_bus: None }
    }

    #[allow(dead_code)]
    pub fn with_event_bus(mut self, event_bus: Arc<dyn EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub(crate) fn publish_event(&self, event: LinkEvent) {
        match Self::encode_event_proto(&event) {
            Ok(payload) => {
                publish_event_fire_and_forget(&self.event_bus, "connector", event.event_type(), payload);
            }
            Err(e) => {
                tracing::error!(error = %e, event_type = %event.event_type(), "Failed to encode event as protobuf");
            }
        }
    }

    /// Encode a LinkEvent as protobuf bytes using the generated proto types.
    fn encode_event_proto(event: &LinkEvent) -> Result<Vec<u8>, String> {
        match event {
            LinkEvent::Created(e) => {
                let proto = platform_proto::connector::MerchantAcquirerLinkCreatedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    connector_id: e.connector_id.clone(),
                    environment: e.environment.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::Enabled(e) => {
                let proto = platform_proto::connector::MerchantAcquirerLinkEnabledEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::Disabled(e) => {
                let proto = platform_proto::connector::MerchantAcquirerLinkDisabledEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::CredentialsRotated(e) => {
                let proto = platform_proto::connector::MerchantAcquirerCredentialsRotatedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    rotated_at_unix_ms: e.rotated_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::ConnectionTested(e) => {
                let proto = platform_proto::connector::ConnectorConnectionTestedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    success: e.success,
                    latency_ms: e.latency_ms,
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::CredentialsExpiring(e) => {
                let proto = platform_proto::connector::ConnectorCredentialsExpiringEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    days_until_expiry: e.days_until_expiry,
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::CredentialsExpired(e) => {
                let proto = platform_proto::connector::ConnectorCredentialsExpiredEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            LinkEvent::HealthChanged(e) => {
                let proto = platform_proto::connector::ConnectorHealthChangedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    old_health: e.old_health.clone(),
                    new_health: e.new_health.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
        }
    }
}

#[async_trait::async_trait]
impl<R: LinkRepository + Send + Sync> CommandHandler for LinkCommandHandler<R> {
    async fn create_link(&self, cmd: CreateLink) -> Result<CreateLinkResult, crate::domain::LinkError> {
        self.create_link_impl(cmd).await
    }

    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, crate::domain::LinkError> {
        self.test_connection_impl(cmd).await
    }

    async fn rotate_credentials(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, crate::domain::LinkError> {
        self.rotate_credentials_impl(cmd).await
    }

    async fn disable_link(&self, cmd: DisableLink) -> Result<DisableLinkResult, crate::domain::LinkError> {
        self.disable_link_impl(cmd).await
    }

    async fn enable_link(&self, cmd: EnableLink) -> Result<EnableLinkResult, crate::domain::LinkError> {
        self.enable_link_impl(cmd).await
    }

    async fn update_metadata(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, crate::domain::LinkError> {
        self.update_metadata_impl(cmd).await
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
}

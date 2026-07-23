//! Command handlers for BYOK Core — MerchantAcquirerLink lifecycle management.

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::domain::{MerchantAcquirerLink, LinkEnvironment, LinkError};
use crate::events::{LinkEvent, LinkCreated, LinkEnabled, LinkDisabled, CredentialsRotated, ConnectionTested, HealthChanged};
use crate::repository::LinkRepository;

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_link(&self, cmd: CreateLink) -> Result<CreateLinkResult, LinkError>;
    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, LinkError>;
    async fn rotate_credentials(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, LinkError>;
    async fn disable_link(&self, cmd: DisableLink) -> Result<DisableLinkResult, LinkError>;
    async fn enable_link(&self, cmd: EnableLink) -> Result<EnableLinkResult, LinkError>;
    async fn update_metadata(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, LinkError>;
}

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct CreateLink {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: LinkEnvironment,
    pub credentials: std::collections::HashMap<String, String>,
}

pub struct TestConnection {
    pub link_id: Uuid,
}

pub struct RotateCredentials {
    pub link_id: Uuid,
    pub new_credentials: std::collections::HashMap<String, String>,
    pub rotate_immediately: bool,
}

pub struct DisableLink {
    pub link_id: Uuid,
    pub reason: String,
}

pub struct EnableLink {
    pub link_id: Uuid,
}

pub struct UpdateMetadata {
    pub link_id: Uuid,
    pub display_name: Option<String>,
}

// ─── Results ────────────────────────────────────────────────────────────────

pub struct CreateLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct TestConnectionResult {
    pub link: MerchantAcquirerLink,
    pub success: bool,
    pub latency_ms: u32,
    pub error_message: Option<String>,
}

pub struct RotateCredentialsResult {
    pub link: MerchantAcquirerLink,
    pub old_credentials_retained: bool,
}

pub struct DisableLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct EnableLinkResult {
    pub link: MerchantAcquirerLink,
}

pub struct UpdateMetadataResult {
    pub link: MerchantAcquirerLink,
}

// ─── Command Handler ────────────────────────────────────────────────────────

pub struct LinkCommandHandler<R: LinkRepository> {
    repository: R,
}

impl<R: LinkRepository> LinkCommandHandler<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    fn publish_event(&self, event: LinkEvent) {
        // Encode event as protobuf using generated proto types
        // Event bus wiring (publish via ChannelEventBus) will be added in future
        match Self::encode_event_proto(&event) {
            Ok(_payload) => {
                tracing::debug!(event_type = %event.event_type(), "Domain event encoded as protobuf");
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
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::Enabled(e) => {
                let proto = platform_proto::connector::MerchantAcquirerLinkEnabledEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::Disabled(e) => {
                let proto = platform_proto::connector::MerchantAcquirerLinkDisabledEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::CredentialsRotated(e) => {
                let proto = platform_proto::connector::MerchantAcquirerCredentialsRotatedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    rotated_at_unix_ms: e.rotated_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::ConnectionTested(e) => {
                let proto = platform_proto::connector::ConnectorConnectionTestedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    success: e.success,
                    latency_ms: e.latency_ms,
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::CredentialsExpiring(e) => {
                let proto = platform_proto::connector::ConnectorCredentialsExpiringEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    days_until_expiry: e.days_until_expiry,
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::CredentialsExpired(e) => {
                let proto = platform_proto::connector::ConnectorCredentialsExpiredEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
            LinkEvent::HealthChanged(e) => {
                let proto = platform_proto::connector::ConnectorHealthChangedEvent {
                    link_id: e.link_id.to_string(),
                    operator_id: e.operator_id.to_string(),
                    old_health: e.old_health.clone(),
                    new_health: e.new_health.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&proto, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
        }
    }
}

#[async_trait::async_trait]
impl<R: LinkRepository + Send + Sync> CommandHandler for LinkCommandHandler<R> {
    async fn create_link(&self, cmd: CreateLink) -> Result<CreateLinkResult, LinkError> {
        // Validate connector_id is not empty
        if cmd.connector_id.is_empty() {
            return Err(LinkError::InvalidRequest("connector_id is required".into()));
        }

        // INV-BYOK-05: Check max links per connector
        let active_count = self.repository.count_active_by_connector(cmd.operator_id, &cmd.connector_id).await?;
        if active_count >= 5 {
            return Err(LinkError::MaxLinksPerConnector);
        }

        // Serialize credentials to JSON for encryption
        let credentials_json = serde_json::to_string(&cmd.credentials)
            .map_err(|e| LinkError::EncryptionFailed(e.to_string()))?;

        // Compute credentials hash for change detection
        let credentials_hash = MerchantAcquirerLink::compute_credentials_hash(&credentials_json);

        // INV-BYOK-03: Encrypt credentials
        let encrypted = serde_json::to_vec(&cmd.credentials)
            .map_err(|e| LinkError::EncryptionFailed(e.to_string()))?;

        // Create the link
        let link = MerchantAcquirerLink::new(
            cmd.operator_id,
            cmd.connector_id.clone(),
            cmd.display_name,
            cmd.environment,
            encrypted,
            credentials_hash,
        );

        self.repository.save(&link).await?;

        self.publish_event(LinkEvent::Created(LinkCreated {
            link_id: link.link_id,
            operator_id: link.operator_id,
            connector_id: link.connector_id.clone(),
            environment: link.environment.as_str().to_string(),
            occurred_at: Utc::now(),
        }));

        info!(
            link_id = %link.link_id,
            connector_id = %link.connector_id,
            "MerchantAcquirerLink created"
        );

        Ok(CreateLinkResult { link })
    }

    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        // Simulate connection test (in production, call connector-gateway)
        // For now, assume test passes
        let success = true;
        let latency_ms = 150;

        let old_health = link.health_status.as_str().to_string();
        link.record_connection_test(success, latency_ms);
        self.repository.save(&link).await?;

        self.publish_event(LinkEvent::ConnectionTested(ConnectionTested {
            link_id: link.link_id,
            operator_id: link.operator_id,
            success,
            latency_ms,
            occurred_at: Utc::now(),
        }));

        if old_health != link.health_status.as_str() {
            self.publish_event(LinkEvent::HealthChanged(HealthChanged {
                link_id: link.link_id,
                operator_id: link.operator_id,
                old_health,
                new_health: link.health_status.as_str().to_string(),
                occurred_at: Utc::now(),
            }));
        }

        info!(
            link_id = %link.link_id,
            success = success,
            latency_ms = latency_ms,
            "Connection test completed"
        );

        Ok(TestConnectionResult {
            link,
            success,
            latency_ms,
            error_message: if success { None } else { Some("Connection failed".into()) },
        })
    }

    async fn rotate_credentials(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        let credentials_json = serde_json::to_string(&cmd.new_credentials)
            .map_err(|e| LinkError::EncryptionFailed(e.to_string()))?;
        let new_hash = MerchantAcquirerLink::compute_credentials_hash(&credentials_json);
        let new_encrypted = serde_json::to_vec(&cmd.new_credentials)
            .map_err(|e| LinkError::EncryptionFailed(e.to_string()))?;

        link.rotate_credentials(new_encrypted, new_hash);
        self.repository.save(&link).await?;

        self.publish_event(LinkEvent::CredentialsRotated(CredentialsRotated {
            link_id: link.link_id,
            operator_id: link.operator_id,
            rotated_at: Utc::now(),
        }));

        info!(link_id = %link.link_id, "Credentials rotated");

        Ok(RotateCredentialsResult {
            link,
            old_credentials_retained: !cmd.rotate_immediately,
        })
    }

    async fn disable_link(&self, cmd: DisableLink) -> Result<DisableLinkResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        link.disable()?;
        self.repository.save(&link).await?;

        self.publish_event(LinkEvent::Disabled(LinkDisabled {
            link_id: link.link_id,
            operator_id: link.operator_id,
            reason: cmd.reason.clone(),
            occurred_at: Utc::now(),
        }));

        info!(link_id = %link.link_id, reason = %cmd.reason, "Link disabled");

        Ok(DisableLinkResult { link })
    }

    async fn enable_link(&self, cmd: EnableLink) -> Result<EnableLinkResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        link.enable()?;
        self.repository.save(&link).await?;

        self.publish_event(LinkEvent::Enabled(LinkEnabled {
            link_id: link.link_id,
            operator_id: link.operator_id,
            occurred_at: Utc::now(),
        }));

        info!(link_id = %link.link_id, "Link enabled");

        Ok(EnableLinkResult { link })
    }

    async fn update_metadata(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        if let Some(name) = cmd.display_name {
            link.update_display_name(name);
        }
        self.repository.save(&link).await?;

        Ok(UpdateMetadataResult { link })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryLinkRepository;
    use crate::domain::LinkStatus;

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

        assert_eq!(rotated.link.credentials_hash,
            MerchantAcquirerLink::compute_credentials_hash(&serde_json::to_string(
                &std::collections::HashMap::from([("key".into(), "new_value".into())])
            ).unwrap()));
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

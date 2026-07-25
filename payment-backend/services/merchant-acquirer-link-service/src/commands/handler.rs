//! Command handlers for BYOK Core — MerchantAcquirerLink lifecycle management.
//!
//! Extracted per CONVENTIONS.md: one concept per file.

pub use super::types::*;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::events::LinkEvent;
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

// ─── Blanket impl: Box<dyn CommandHandler> delegates to inner ────────────────

#[async_trait::async_trait]
impl<T: CommandHandler + ?Sized> CommandHandler for Box<T> {
    async fn create_link(&self, cmd: CreateLink) -> Result<CreateLinkResult, crate::domain::LinkError> {
        (**self).create_link(cmd).await
    }

    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, crate::domain::LinkError> {
        (**self).test_connection(cmd).await
    }

    async fn rotate_credentials(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, crate::domain::LinkError> {
        (**self).rotate_credentials(cmd).await
    }

    async fn disable_link(&self, cmd: DisableLink) -> Result<DisableLinkResult, crate::domain::LinkError> {
        (**self).disable_link(cmd).await
    }

    async fn enable_link(&self, cmd: EnableLink) -> Result<EnableLinkResult, crate::domain::LinkError> {
        (**self).enable_link(cmd).await
    }

    async fn update_metadata(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, crate::domain::LinkError> {
        (**self).update_metadata(cmd).await
    }
}


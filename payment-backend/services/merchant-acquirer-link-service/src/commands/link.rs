//! Link lifecycle command handlers (create, enable, disable, update metadata).

use chrono::Utc;
use tracing::info;

use crate::domain::{MerchantAcquirerLink, LinkError};
use crate::events::{LinkEvent, LinkCreated, LinkEnabled, LinkDisabled};
use crate::repository::LinkRepository;
use super::types::*;
use super::LinkCommandHandler;

impl<R: LinkRepository + Send + Sync> LinkCommandHandler<R> {
    pub(crate) async fn create_link_impl(&self, cmd: CreateLink) -> Result<CreateLinkResult, LinkError> {
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

    pub(crate) async fn disable_link_impl(&self, cmd: DisableLink) -> Result<DisableLinkResult, LinkError> {
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

    pub(crate) async fn enable_link_impl(&self, cmd: EnableLink) -> Result<EnableLinkResult, LinkError> {
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

    pub(crate) async fn update_metadata_impl(&self, cmd: UpdateMetadata) -> Result<UpdateMetadataResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        if let Some(name) = cmd.display_name {
            link.update_display_name(name);
        }
        self.repository.save(&link).await?;

        Ok(UpdateMetadataResult { link })
    }
}

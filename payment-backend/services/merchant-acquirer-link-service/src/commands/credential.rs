//! Credential rotation and connection test command handlers.

use chrono::Utc;
use tracing::info;

use crate::domain::{MerchantAcquirerLink, LinkError};
use crate::events::{LinkEvent, CredentialsRotated, ConnectionTested, HealthChanged};
use crate::repository::LinkRepository;
use super::types::*;
use super::LinkCommandHandler;

impl<R: LinkRepository + Send + Sync> LinkCommandHandler<R> {
    pub(crate) async fn rotate_credentials_impl(&self, cmd: RotateCredentials) -> Result<RotateCredentialsResult, LinkError> {
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

    pub(crate) async fn test_connection_impl(&self, cmd: TestConnection) -> Result<TestConnectionResult, LinkError> {
        let mut link = self.repository.load(cmd.link_id).await?
            .ok_or(LinkError::NotFound(cmd.link_id))?;

        // Simulate connection test (in production, call connector-gateway)
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
}

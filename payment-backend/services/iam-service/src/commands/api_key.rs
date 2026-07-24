//! API key command handlers.

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::domain::{ApiKey, ApiKeyStatus, IamError};
use crate::events::{IamEvent, ApiKeyCreated, ApiKeyRevoked};
use crate::repository::IamRepository;
use super::types::*;
use super::IamCommandHandler;

impl<R: IamRepository + Send + Sync> IamCommandHandler<R> {
    pub(crate) async fn create_api_key_impl(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, IamError> {
        // Verify principal exists
        self.repository.load_principal(cmd.principal_id).await?
            .ok_or(IamError::PrincipalNotFound(cmd.principal_id))?;

        // Check for duplicate name
        if self.repository.find_api_key_by_name(cmd.principal_id, &cmd.name).await?.is_some() {
            return Err(IamError::DuplicateApiKeyName(cmd.name));
        }

        let api_key_id = Uuid::now_v7();
        let api_key_secret = Uuid::now_v7().to_string();
        let expires_at = cmd.expires_in_days
            .unwrap_or(90)
            .clamp(1, 365);

        let key = ApiKey {
            api_key_id,
            principal_id: cmd.principal_id,
            name: cmd.name,
            key_hash: self.hash_key(&api_key_secret),
            scopes: cmd.scopes.clone(),
            status: ApiKeyStatus::Active,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(expires_at as i64)),
            last_used_at: None,
        };

        self.repository.save_api_key(&key).await?;

        self.publish_event(IamEvent::ApiKeyCreated(ApiKeyCreated {
            api_key_id,
            principal_id: cmd.principal_id,
            scopes: cmd.scopes,
            occurred_at: Utc::now(),
        }));

        info!(api_key_id = %api_key_id, "API key created");

        Ok(CreateApiKeyResult {
            api_key: key,
            api_key_secret,
        })
    }

    pub(crate) async fn revoke_api_key_impl(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, IamError> {
        let mut key = self.repository.load_api_key(cmd.api_key_id).await?
            .ok_or_else(|| IamError::InvalidRequest("API key not found".into()))?;

        if key.principal_id != cmd.principal_id {
            return Err(IamError::AuthorizationDenied("Principal does not own this API key".into()));
        }

        key.status = ApiKeyStatus::Revoked;
        self.repository.save_api_key(&key).await?;

        self.publish_event(IamEvent::ApiKeyRevoked(ApiKeyRevoked {
            api_key_id: cmd.api_key_id,
            principal_id: cmd.principal_id,
            occurred_at: Utc::now(),
        }));

        info!(api_key_id = %cmd.api_key_id, "API key revoked");

        Ok(RevokeApiKeyResult { revoked: true })
    }
}

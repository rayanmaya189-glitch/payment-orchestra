//! Command handlers for BC-02 Identity & Access Management.
//!
//! Extracted per CONVENTIONS.md: one concept per file.

pub use super::types::*;

use chrono::Utc;
use uuid::Uuid;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::events::IamEvent;
use crate::repository::IamRepository;

// Blanket impl: Box<dyn CommandHandler> implements CommandHandler
#[async_trait::async_trait]
impl CommandHandler for Box<dyn CommandHandler> {
    async fn authenticate(&self, cmd: Authenticate) -> Result<AuthenticateResult, crate::domain::IamError> {
        self.as_ref().authenticate(cmd).await
    }
    async fn create_api_key(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, crate::domain::IamError> {
        self.as_ref().create_api_key(cmd).await
    }
    async fn revoke_api_key(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, crate::domain::IamError> {
        self.as_ref().revoke_api_key(cmd).await
    }
    async fn submit_change(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, crate::domain::IamError> {
        self.as_ref().submit_change(cmd).await
    }
    async fn review_change(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, crate::domain::IamError> {
        self.as_ref().review_change(cmd).await
    }
}

// ─── Command Trait ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn authenticate(&self, cmd: Authenticate) -> Result<AuthenticateResult, crate::domain::IamError>;
    async fn create_api_key(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, crate::domain::IamError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, crate::domain::IamError>;
    async fn submit_change(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, crate::domain::IamError>;
    async fn review_change(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, crate::domain::IamError>;
}

// ─── Command Handler Implementation ────────────────────────────────────────

pub struct IamCommandHandler<R: IamRepository> {
    pub(crate) repository: R,
    pub(crate) jwt_secret: String,
    pub(crate) event_bus: Option<Arc<dyn EventBus>>,
}

impl<R: IamRepository> IamCommandHandler<R> {
    pub fn new(repository: R, jwt_secret: String) -> Self {
        Self { repository, jwt_secret, event_bus: None }
    }

    #[allow(dead_code)]
    pub fn with_event_bus(mut self, event_bus: Arc<dyn EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub(crate) fn publish_event(&self, event: IamEvent) {
        match Self::encode_event_proto(&event) {
            Ok(payload) => {
                publish_event_fire_and_forget(&self.event_bus, "iam", event.event_type(), payload);
            }
            Err(e) => {
                tracing::error!(error = %e, event_type = %event.event_type(), "Failed to encode event as protobuf");
            }
        }
    }

    /// Encode an IamEvent as protobuf bytes using the generated proto types.
    fn encode_event_proto(event: &IamEvent) -> Result<Vec<u8>, String> {
        match event {
            IamEvent::PrincipalCreated(e) => {
                let proto = platform_proto::iam::PrincipalCreatedEvent {
                    principal_id: e.principal_id.to_string(),
                    email: e.email.clone(),
                    principal_type: e.principal_type.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            IamEvent::PrincipalAuthenticated(e) => {
                let proto = platform_proto::iam::PrincipalAuthenticatedEvent {
                    principal_id: e.principal_id.to_string(),
                    ip_address: e.ip_address.clone(),
                    user_agent: e.user_agent.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            IamEvent::PermissionDenied(e) => {
                let proto = platform_proto::iam::PermissionDeniedEvent {
                    principal_id: e.principal_id.to_string(),
                    resource: e.resource.clone(),
                    action: e.action.clone(),
                    reason: e.reason.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            IamEvent::ApiKeyCreated(e) => {
                let proto = platform_proto::iam::ApiKeyCreatedEvent {
                    api_key_id: e.api_key_id.to_string(),
                    principal_id: e.principal_id.to_string(),
                    scopes: e.scopes.clone(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
            IamEvent::ApiKeyRevoked(e) => {
                let proto = platform_proto::iam::ApiKeyRevokedEvent {
                    api_key_id: e.api_key_id.to_string(),
                    principal_id: e.principal_id.to_string(),
                    occurred_at_unix_ms: e.occurred_at.timestamp_millis(),
                };
                encode_proto!(proto)
            }
        }
    }

    /// Simplified password verification — use Argon2id in production
    pub(crate) fn verify_password(&self, password: &str, hash: &[u8]) -> bool {
        if hash.is_empty() {
            return false;
        }
        let computed = ring::digest::digest(&ring::digest::SHA256, password.as_bytes());
        computed.as_ref() == hash
    }

    /// Generate a signed token (simplified — use proper JWT in production)
    pub(crate) fn generate_token(&self, principal_id: &Uuid, token_type: &str, expiry_seconds: u64) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let payload = format!("{}:{}:{}:{}", principal_id, token_type, Utc::now().timestamp(), expiry_seconds);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.jwt_secret.as_bytes())
            .expect("HMAC key");
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        format!("{}.{}", hex::encode(payload.as_bytes()), signature)
    }

    /// Hash an API key secret for storage (SHA-256 for dev, Argon2id for prod)
    pub(crate) fn hash_key(&self, key: &str) -> Vec<u8> {
        let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
        digest.as_ref().to_vec()
    }
}

#[async_trait::async_trait]
impl<R: IamRepository + Send + Sync> CommandHandler for IamCommandHandler<R> {
    async fn authenticate(&self, cmd: Authenticate) -> Result<AuthenticateResult, crate::domain::IamError> {
        self.authenticate_impl(cmd).await
    }

    async fn create_api_key(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, crate::domain::IamError> {
        self.create_api_key_impl(cmd).await
    }

    async fn revoke_api_key(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, crate::domain::IamError> {
        self.revoke_api_key_impl(cmd).await
    }

    async fn submit_change(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, crate::domain::IamError> {
        self.submit_change_impl(cmd).await
    }

    async fn review_change(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, crate::domain::IamError> {
        self.review_change_impl(cmd).await
    }
}


//! Command handlers for BC-02 Identity & Access Management.

pub mod types;
pub(crate) mod auth;
pub(crate) mod api_key;
pub(crate) mod maker_checker;

pub use types::*;

use chrono::Utc;
use uuid::Uuid;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::events::IamEvent;
use crate::repository::IamRepository;

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

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::domain::{ApiKeyStatus, ChangeStatus, Principal};
    use crate::repository::InMemoryIamRepository;

    fn create_handler() -> IamCommandHandler<InMemoryIamRepository> {
        IamCommandHandler::new(InMemoryIamRepository::new(), "test-secret".into())
    }

    fn create_test_principal(repo: &InMemoryIamRepository) -> Principal {
        let principal = Principal::new_human(
            Uuid::now_v7(),
            "admin@test.com".into(),
            ring::digest::digest(&ring::digest::SHA256, b"password123").as_ref().to_vec(),
        );
        let _ = futures::executor::block_on(repo.save_principal(&principal));
        principal
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let repo = InMemoryIamRepository::new();
        create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let result = handler.authenticate(Authenticate {
            email: "admin@test.com".into(),
            password: "password123".into(),
            ip_address: "1.2.3.4".parse().unwrap(),
            user_agent: "test-agent".into(),
        }).await.unwrap();

        assert!(!result.access_token.is_empty());
        assert!(!result.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let repo = InMemoryIamRepository::new();
        create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let result = handler.authenticate(Authenticate {
            email: "admin@test.com".into(),
            password: "wrong-password".into(),
            ip_address: "1.2.3.4".parse().unwrap(),
            user_agent: "test-agent".into(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_api_key_success() {
        let repo = InMemoryIamRepository::new();
        let principal = create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let result = handler.create_api_key(CreateApiKey {
            principal_id: principal.id,
            name: "Production Key".into(),
            scopes: vec!["payments:read".into(), "payments:write".into()],
            expires_in_days: Some(90),
        }).await.unwrap();

        assert_eq!(result.api_key.status, ApiKeyStatus::Active);
        assert!(!result.api_key_secret.is_empty());
    }

    #[tokio::test]
    async fn test_create_duplicate_api_key_rejected() {
        let repo = InMemoryIamRepository::new();
        let principal = create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        handler.create_api_key(CreateApiKey {
            principal_id: principal.id,
            name: "Production Key".into(),
            scopes: vec!["payments:read".into()],
            expires_in_days: Some(90),
        }).await.unwrap();

        let result = handler.create_api_key(CreateApiKey {
            principal_id: principal.id,
            name: "Production Key".into(),
            scopes: vec!["payments:read".into()],
            expires_in_days: Some(90),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_api_key() {
        let repo = InMemoryIamRepository::new();
        let principal = create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo.clone(), "test-secret".into());

        let created = handler.create_api_key(CreateApiKey {
            principal_id: principal.id,
            name: "Key to Revoke".into(),
            scopes: vec!["payments:read".into()],
            expires_in_days: Some(90),
        }).await.unwrap();

        let result = handler.revoke_api_key(RevokeApiKey {
            api_key_id: created.api_key.api_key_id,
            principal_id: principal.id,
        }).await.unwrap();

        assert!(result.revoked);
    }

    #[tokio::test]
    async fn test_submit_and_approve_change() {
        let repo = InMemoryIamRepository::new();
        let maker_id = Uuid::now_v7();
        let checker_id = Uuid::now_v7();
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let submitted = handler.submit_change(SubmitChange {
            change_type: "update_gateway_config".into(),
            maker_id,
            payload: vec![1, 2, 3],
            maker_note: Some("Update fee structure".into()),
        }).await.unwrap();

        assert_eq!(submitted.change.status, ChangeStatus::Pending);

        let reviewed = handler.review_change(ReviewChange {
            change_id: submitted.change.change_id,
            checker_id,
            approved: true,
            checker_note: Some("Approved".into()),
        }).await.unwrap();

        assert_eq!(reviewed.change.status, ChangeStatus::Approved);
    }

    #[tokio::test]
    async fn test_self_approval_rejected() {
        let repo = InMemoryIamRepository::new();
        let maker_id = Uuid::now_v7();
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let submitted = handler.submit_change(SubmitChange {
            change_type: "test".into(),
            maker_id,
            payload: vec![],
            maker_note: None,
        }).await.unwrap();

        let result = handler.review_change(ReviewChange {
            change_id: submitted.change.change_id,
            checker_id: maker_id,
            approved: true,
            checker_note: None,
        }).await;

        assert!(result.is_err());
    }
}

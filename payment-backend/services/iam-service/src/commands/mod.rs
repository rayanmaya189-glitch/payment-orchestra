//! Command handlers for BC-02 Identity & Access Management.

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::domain::{Principal, ApiKey, ApiKeyStatus, PendingChange, AuthError, IamError};
use crate::events::{IamEvent, PrincipalAuthenticated, PermissionDenied, ApiKeyCreated, ApiKeyRevoked};
use crate::repository::IamRepository;

// ─── Command Trait ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn authenticate(&self, cmd: Authenticate) -> Result<AuthenticateResult, IamError>;
    async fn create_api_key(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, IamError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, IamError>;
    async fn submit_change(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, IamError>;
    async fn review_change(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, IamError>;
}

// ─── Command Types ──────────────────────────────────────────────────────────

pub struct Authenticate {
    pub email: String,
    pub password: String,
    pub ip_address: std::net::IpAddr,
    pub user_agent: String,
}

pub struct CreateApiKey {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

pub struct RevokeApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
}

pub struct SubmitChange {
    pub change_type: String,
    pub maker_id: Uuid,
    pub payload: Vec<u8>,
    pub maker_note: Option<String>,
}

pub struct ReviewChange {
    pub change_id: Uuid,
    pub checker_id: Uuid,
    pub approved: bool,
    pub checker_note: Option<String>,
}

// ─── Results ────────────────────────────────────────────────────────────────

pub struct AuthenticateResult {
    pub principal: Principal,
    pub access_token: String,
    pub refresh_token: String,
    pub mfa_required: bool,
    pub mfa_method: Option<String>,
}

pub struct CreateApiKeyResult {
    pub api_key: ApiKey,
    pub api_key_secret: String,
}

pub struct RevokeApiKeyResult {
    pub revoked: bool,
}

pub struct SubmitChangeResult {
    pub change: PendingChange,
}

pub struct ReviewChangeResult {
    pub change: PendingChange,
}

// ─── Command Handler Implementation ────────────────────────────────────────

pub struct IamCommandHandler<R: IamRepository> {
    repository: R,
    jwt_secret: String,
    event_bus: Option<Arc<dyn EventBus>>,
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

    fn publish_event(&self, event: IamEvent) {
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
}

#[async_trait::async_trait]
impl<R: IamRepository + Send + Sync> CommandHandler for IamCommandHandler<R> {
    async fn authenticate(&self, cmd: Authenticate) -> Result<AuthenticateResult, IamError> {
        let principal = self.repository.find_principal_by_email(&cmd.email).await?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check account status
        principal.can_authenticate()?;

        // Verify password (simplified — use Argon2id in production)
        let password_valid = self.verify_password(&cmd.password, principal.password_hash.as_deref().unwrap_or_default());
        if !password_valid {
            let mut p = principal.clone();
            // Save first, then check lock — ensures locked state is persisted
            // before propagating any lock error to the caller
            let login_result = p.record_login_attempt(false);
            self.repository.save_principal(&p).await?;

            if let Err(e) = login_result {
                // Account is now locked — return the lock error
                return Err(e.into());
            }

            self.publish_event(IamEvent::PermissionDenied(PermissionDenied {
                principal_id: p.id,
                resource: "authentication".into(),
                action: "login".into(),
                reason: "Invalid credentials".into(),
                occurred_at: Utc::now(),
            }));
            return Err(AuthError::InvalidCredentials.into());
        }

        // Record successful login
        let mut p = principal.clone();
        p.record_login_attempt(true)?;
        self.repository.save_principal(&p).await?;

        // Generate tokens
        let access_token = self.generate_token(&p.id, "access", 3600);
        let refresh_token = self.generate_token(&p.id, "refresh", 86400 * 30);

        self.publish_event(IamEvent::PrincipalAuthenticated(PrincipalAuthenticated {
            principal_id: p.id,
            ip_address: cmd.ip_address.to_string(),
            user_agent: cmd.user_agent,
            occurred_at: Utc::now(),
        }));

        info!(principal_id = %p.id, "Principal authenticated");

        Ok(AuthenticateResult {
            principal: p,
            access_token,
            refresh_token,
            mfa_required: principal.mfa_enrolled,
            mfa_method: principal.mfa_method.as_ref().map(|m| m.as_str().to_string()),
        })
    }

    async fn create_api_key(&self, cmd: CreateApiKey) -> Result<CreateApiKeyResult, IamError> {
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
            .min(365)
            .max(1);

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

    async fn revoke_api_key(&self, cmd: RevokeApiKey) -> Result<RevokeApiKeyResult, IamError> {
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

    async fn submit_change(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, IamError> {
        let change = PendingChange::new(
            cmd.change_type,
            cmd.maker_id,
            cmd.payload,
            cmd.maker_note,
        );

        self.repository.save_change(&change).await?;

        info!(change_id = %change.change_id, type = %change.change_type, "Pending change submitted");

        Ok(SubmitChangeResult { change })
    }

    async fn review_change(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, IamError> {
        let mut change = self.repository.load_change(cmd.change_id).await?
            .ok_or_else(|| IamError::InvalidRequest("Change not found".into()))?;

        if cmd.approved {
            change.approve(cmd.checker_id, cmd.checker_note)?;
        } else {
            change.reject(cmd.checker_id, cmd.checker_note)?;
        }

        self.repository.save_change(&change).await?;

        info!(change_id = %change.change_id, status = %change.status.as_str(), "Change reviewed");

        Ok(ReviewChangeResult { change })
    }
}

// ─── Private Helpers ─────────────────────────────────────────────────────

impl<R: IamRepository> IamCommandHandler<R> {
    /// Simplified password verification — use Argon2id in production
    fn verify_password(&self, password: &str, hash: &[u8]) -> bool {
        if hash.is_empty() {
            return false;
        }
        let computed = ring::digest::digest(&ring::digest::SHA256, password.as_bytes());
        computed.as_ref() == hash
    }

    /// Generate a signed token (simplified — use proper JWT in production)
    fn generate_token(&self, principal_id: &Uuid, token_type: &str, expiry_seconds: u64) -> String {
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
    fn hash_key(&self, key: &str) -> Vec<u8> {
        let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
        digest.as_ref().to_vec()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ChangeStatus;
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

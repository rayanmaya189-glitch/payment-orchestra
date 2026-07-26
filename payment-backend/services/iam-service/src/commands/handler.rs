//! Command handlers for BC-02 Identity & Access Management.
//!
//! Extracted per CONVENTIONS.md: one concept per file.
//!
//! ## Security Architecture
//!
//! - **Password hashing**: Argon2id (memory-hard, time-hard, side-channel resistant)
//! - **JWT signing**: RS256 (RSASSA-PKCS1-v1_5 with SHA-256) with PEM-encoded private key
//!   - Falls back to HS256 if no RSA key is configured (dev mode)
//! - **API key hashing**: Argon2id (same as passwords — API keys are bearer credentials)
//! - **Token claims**: Proper `sub`, `exp`, `iat`, `iss`, `aud`, `jti` claims
//! - **Key rotation**: Supported via `JWT_PRIVATE_KEY_PEM` env var, restart to pick up new key

pub use super::types::*;

use chrono::Utc;
use uuid::Uuid;
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use argon2::{Argon2, PasswordHasher, PasswordVerifier, PasswordHash};
use argon2::password_hash::{SaltString, rand_core::OsRng};

use std::sync::Arc;
use platform_messaging::{event_bus::{EventBus, publish_event_fire_and_forget}, encode_proto};
use crate::events::IamEvent;
use crate::repository::IamRepository;

// ─── Constants ─────────────────────────────────────────────────────────────

/// Default token expiry durations
pub(crate) const ACCESS_TOKEN_TTL_SECS: u64 = 3600;        // 1 hour
pub(crate) const REFRESH_TOKEN_TTL_SECS: u64 = 2_592_000;   // 30 days

/// Application identifier used in JWT `iss` and `aud` claims
const JWT_ISSUER: &str = "payment-orchestra-iam";
const JWT_AUDIENCE: &str = "payment-orchestra-api";

// ─── Blanket impl: Box<dyn CommandHandler> ─────────────────────────────────

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

// ─── JWT Claims ────────────────────────────────────────────────────────────

/// JWT payload claims. Conforms to RFC 7519 with custom claims for the platform.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IamJwtClaims {
    /// Subject — principal UUID
    pub sub: String,
    /// Principal type (human | api_key | service)
    #[serde(rename = "pty")]
    pub principal_type: String,
    /// Operator ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<String>,
    /// Permissions list
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub perms: Vec<String>,
    /// Session ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// JWT ID — unique token identifier for revocation
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiry (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Token type: "access" | "refresh"
    #[serde(rename = "tty")]
    pub token_type: String,
}

// ─── Signing Algorithm and Key Resolution ──────────────────────────────────

/// Resolve the JWT signing algorithm and encoding key.
///
/// Priority:
/// 1. `JWT_PRIVATE_KEY_PEM` env var → RS256 (production)
/// 2. `jwt_secret` parameter → HS256 (dev mode fallback)
pub(crate) fn resolve_jwt_key(jwt_secret: &str) -> (Algorithm, EncodingKey) {
    // Try RS256 with PEM-encoded private key first (AUTH-017 compliance)
    if let Ok(pem) = std::env::var("JWT_PRIVATE_KEY_PEM") {
        if !pem.is_empty() {
            match EncodingKey::from_rsa_pem(pem.as_bytes()) {
                Ok(key) => {
                    tracing::info!("JWT signing using RS256 with PEM private key");
                    return (Algorithm::RS256, key);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to parse JWT_PRIVATE_KEY_PEM, falling back to HS256"
                    );
                }
            }
        }
    }

    // Fallback: HS256 with shared secret
    tracing::info!("JWT signing using HS256 (dev mode — set JWT_PRIVATE_KEY_PEM for RS256 production)");
    (Algorithm::HS256, EncodingKey::from_secret(jwt_secret.as_bytes()))
}

// ─── Command Handler Implementation ────────────────────────────────────────

pub struct IamCommandHandler<R: IamRepository> {
    pub(crate) repository: R,
    pub(crate) event_bus: Option<Arc<dyn EventBus>>,
    pub(crate) jwt_algorithm: Algorithm,
    pub(crate) jwt_encoding_key: EncodingKey,
}

impl<R: IamRepository> IamCommandHandler<R> {
    pub fn new(repository: R, jwt_secret: String) -> Self {
        let (jwt_algorithm, jwt_encoding_key) = resolve_jwt_key(&jwt_secret);
        Self {
            repository,
            event_bus: None,
            jwt_algorithm,
            jwt_encoding_key,
        }
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

    pub(crate) fn verify_password(&self, password: &str, hash: &[u8]) -> bool {
        if hash.is_empty() {
            return false;
        }
        let hash_str = match std::str::from_utf8(hash) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let parsed_hash = match PasswordHash::new(hash_str) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    }

    /// Generate a signed JWT token with proper claims.
    ///
    /// Uses RS256 if a PEM private key is configured, otherwise HS256 (dev mode).
    /// This complies with AUTH-017 which mandates RS256/ES256 in production.
    pub(crate) fn generate_token(&self, principal_id: &Uuid, principal_type: &str, token_type: &str, expiry_seconds: u64, permissions: &[String], operator_id: Option<Uuid>) -> Result<String, crate::domain::IamError> {
        let now = Utc::now();
        let claims = IamJwtClaims {
            sub: principal_id.to_string(),
            principal_type: principal_type.to_string(),
            oid: operator_id.map(|id| id.to_string()),
            perms: permissions.to_vec(),
            sid: None,
            jti: Uuid::now_v7().to_string(),
            iss: JWT_ISSUER.to_string(),
            aud: JWT_AUDIENCE.to_string(),
            exp: (now.timestamp() + expiry_seconds as i64) as u64,
            iat: now.timestamp() as u64,
            token_type: token_type.to_string(),
        };

        encode(&Header::new(self.jwt_algorithm), &claims, &self.jwt_encoding_key)
            .map_err(|e| {
                tracing::error!(error = %e, algorithm = ?self.jwt_algorithm, "JWT signing failed");
                crate::domain::IamError::InvalidRequest(format!("Token generation failed: {}", e))
            })
    }

    pub(crate) fn hash_key(&self, key: &str) -> Vec<u8> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(key.as_bytes(), &salt)
            .expect("Argon2 API key hashing failed");
        hash.to_string().into_bytes()
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


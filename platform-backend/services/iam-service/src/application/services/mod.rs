use async_trait::async_trait;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString, PasswordVerifier, PasswordHash};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::LoginResponse;
use crate::domain::aggregates::{ApiKey, PrincipalStatus};
use crate::domain::value_objects::{PermissionContext, PermissionResult};
use crate::infrastructure::repository::{ApiKeyRepository, PrincipalRepository};
use platform_error::PlatformError;

const JWT_SECRET: &[u8] = b"platform-secret-change-in-production";
const JWT_EXPIRY_HOURS: i64 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub role: String,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn authenticate(&self, cmd: AuthenticateCommand) -> Result<LoginResponse, PlatformError>;
    async fn issue_token(&self, cmd: IssueTokenCommand) -> Result<LoginResponse, PlatformError>;
    async fn validate_permission(&self, query: ValidatePermissionQuery) -> Result<PermissionResult, PlatformError>;
    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<ApiKeyResult, PlatformError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError>;
    async fn approve_pending_change(&self, cmd: ApprovePendingChangeCommand) -> Result<(), PlatformError>;
}

pub struct AuthServiceImpl {
    principal_repo: Box<dyn PrincipalRepository>,
    api_key_repo: Box<dyn ApiKeyRepository>,
    db: sea_orm::DatabaseConnection,
}

impl AuthServiceImpl {
    pub fn new(
        principal_repo: Box<dyn PrincipalRepository>,
        api_key_repo: Box<dyn ApiKeyRepository>,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        Self { principal_repo, api_key_repo, db }
    }

    fn hash_password(password: &str) -> Result<Vec<u8>, PlatformError> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PlatformError::Internal(format!("Password hash error: {e}")))?;
        Ok(hash.to_string().into_bytes())
    }

    fn verify_password(password: &str, hash: &[u8]) -> Result<bool, PlatformError> {
        let hash_str = std::str::from_utf8(hash)
            .map_err(|e| PlatformError::Internal(format!("Invalid hash encoding: {e}")))?;
        let parsed_hash = PasswordHash::new(hash_str)
            .map_err(|e| PlatformError::Internal(format!("Invalid hash: {e}")))?;
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    fn generate_jwt(principal_id: Uuid, role: &str) -> Result<String, PlatformError> {
        let now = Utc::now();
        let claims = Claims {
            sub: principal_id.to_string(),
            exp: (now + Duration::hours(JWT_EXPIRY_HOURS)).timestamp() as usize,
            iat: now.timestamp() as usize,
            role: role.to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET),
        )
        .map_err(|e| PlatformError::Internal(format!("JWT error: {e}")))
    }

    fn generate_refresh_token() -> String {
        Uuid::now_v7().to_string()
    }

    fn hash_api_key(key: &str) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[derive(Debug)]
pub struct ApiKeyResult {
    pub api_key_id: String,
    pub api_key_secret: String,
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(&self, cmd: AuthenticateCommand) -> Result<LoginResponse, PlatformError> {
        // Find principal by email
        let mut principal = self.principal_repo
            .find_by_email(&cmd.email)
            .await?
            .ok_or_else(|| PlatformError::AuthorizationDenied("Invalid credentials".into()))?;

        // Check if account is locked (AUTH-007)
        if principal.is_locked() {
            return Err(PlatformError::AuthorizationDenied(format!(
                "Account locked until {:?}",
                principal.locked_until
            )));
        }

        // Verify password
        let password_hash = principal.password_hash.as_ref()
            .ok_or_else(|| PlatformError::AuthorizationDenied("No password set".into()))?;

        if !Self::verify_password(&cmd.password, password_hash)? {
            principal.record_failed_login();
            self.principal_repo.save(&principal).await?;
            return Err(PlatformError::AuthorizationDenied("Invalid credentials".into()));
        }

        // Check status
        if principal.status != PrincipalStatus::Active {
            return Err(PlatformError::AuthorizationDenied("Account not active".into()));
        }

        // Record successful login
        principal.record_successful_login();
        self.principal_repo.save(&principal).await?;

        // Get role
        let roles = self.principal_repo.list_roles(principal.id).await?;
        let role = roles.first().map(|r| r.role_name.as_str()).unwrap_or("readonly");

        // Generate tokens
        let access_token = Self::generate_jwt(principal.id, role)?;
        let refresh_token = Self::generate_refresh_token();

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: (JWT_EXPIRY_HOURS * 3600) as u64,
        })
    }

    async fn issue_token(&self, cmd: IssueTokenCommand) -> Result<LoginResponse, PlatformError> {
        let _ = cmd;
        Err(PlatformError::Internal("Refresh token rotation not implemented".into()))
    }

    async fn validate_permission(&self, query: ValidatePermissionQuery) -> Result<PermissionResult, PlatformError> {
        let principal = self.principal_repo
            .load(query.principal_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Principal".into(),
                id: query.principal_id,
            })?;

        // Get roles
        let roles = self.principal_repo.list_roles(principal.id).await?;
        let role = roles.first().map(|r| r.role_name.as_str()).unwrap_or("readonly");

        // ABAC evaluation
        let allowed = match (role, query.resource.as_str(), query.action.as_str()) {
            ("admin", _, _) => true,
            ("finance", "payment_intents", "capture") => true,
            ("finance", "payment_intents", "refund") => {
                if let Some(amount) = query.context.amount {
                    amount <= 50000
                } else {
                    true
                }
            }
            ("developer", "payment_intents", "read") => true,
            ("developer", "invoices", "read") => true,
            ("readonly", _, "read") => true,
            _ => false,
        };

        let requires_maker_checker = !allowed && matches!(role, "finance" | "developer");

        Ok(PermissionResult {
            allowed,
            requires_maker_checker,
        })
    }

    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<ApiKeyResult, PlatformError> {
        let api_key_secret = format!("pk_{}", Uuid::now_v7());
        let key_hash = Self::hash_api_key(&api_key_secret);

        let expires_in_days = cmd.expires_in_days.unwrap_or(90);
        let expires_at = Utc::now() + Duration::days(expires_in_days as i64);

        let api_key = ApiKey {
            id: Uuid::now_v7(),
            principal_id: cmd.principal_id,
            name: cmd.name,
            key_hash,
            scopes: cmd.scopes,
            acquirer_link_ids: cmd.acquirer_link_ids,
            expires_at,
            revoked_at: None,
            created_at: Utc::now(),
        };

        self.api_key_repo.save(&api_key).await?;

        Ok(ApiKeyResult {
            api_key_id: api_key.id.to_string(),
            api_key_secret,
        })
    }

    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError> {
        self.api_key_repo.revoke(cmd.api_key_id).await
    }

    async fn approve_pending_change(&self, cmd: ApprovePendingChangeCommand) -> Result<(), PlatformError> {
        let _ = cmd;
        Ok(())
    }
}

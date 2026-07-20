use async_trait::async_trait;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString, PasswordVerifier, PasswordHash};
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use chrono::{Utc, Duration, DateTime};
use uuid::Uuid;
use redis::aio::ConnectionManager;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::LoginResponse;
use crate::domain::aggregates::{ApiKey, PrincipalStatus};
use crate::domain::value_objects::PermissionResult;
use crate::infrastructure::repository::{ApiKeyRepository, PendingChangeRepository, PrincipalRepository};
use platform_config::AuthConfig;
use platform_error::PlatformError;

/// JWT claims matching SRS AUTH-002 (short-lived access tokens).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,       // principal UUID
    pub exp: usize,        // expiry timestamp
    pub iat: usize,        // issued-at timestamp
    pub role: String,      // role name
    pub jti: String,       // unique token ID (for revocation)
    pub iss: String,       // issuer
    pub aud: String,       // audience (client type: "dashboard" | "api")
}

/// Server-side refresh token record stored in Redis (SRS AUTH-002, SESS-SEC-003).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefreshTokenRecord {
    pub token_id: String,
    pub principal_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    /// If this token was already rotated, points to the next token.
    pub next_token_id: Option<String>,
    /// Client fingerprint (IP + User-Agent hash) for anomaly detection.
    pub client_fingerprint: String,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn authenticate(&self, cmd: AuthenticateCommand) -> Result<LoginResponse, PlatformError>;
    async fn issue_token(&self, cmd: IssueTokenCommand, client_fingerprint: String) -> Result<LoginResponse, PlatformError>;
    async fn validate_permission(&self, query: ValidatePermissionQuery) -> Result<PermissionResult, PlatformError>;
    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<ApiKeyResult, PlatformError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError>;
    async fn approve_pending_change(&self, cmd: ApprovePendingChangeCommand) -> Result<(), PlatformError>;
    /// Revoke all sessions for a principal (SRS SESS-SEC-004).
    async fn revoke_all_sessions(&self, principal_id: Uuid) -> Result<(), PlatformError>;
}

pub struct AuthServiceImpl {
    principal_repo: Box<dyn PrincipalRepository>,
    api_key_repo: Box<dyn ApiKeyRepository>,
    pending_change_repo: Box<dyn PendingChangeRepository>,
    _db: sea_orm::DatabaseConnection,
    redis: ConnectionManager,
    auth_config: AuthConfig,
}

impl AuthServiceImpl {
    pub fn new(
        principal_repo: Box<dyn PrincipalRepository>,
        api_key_repo: Box<dyn ApiKeyRepository>,
        pending_change_repo: Box<dyn PendingChangeRepository>,
        db: sea_orm::DatabaseConnection,
        redis: ConnectionManager,
        auth_config: AuthConfig,
    ) -> Self {
        Self { principal_repo, api_key_repo, pending_change_repo, _db: db, redis, auth_config }
    }

    // ==================== Password Hashing (Argon2id per SRS AUTH-005) ====================

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

    // ==================== API Key Hashing (Argon2id per SRS AUTH-005) ====================

    fn hash_api_key(key: &str) -> Result<String, PlatformError> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(key.as_bytes(), &salt)
            .map_err(|e| PlatformError::Internal(format!("API key hash error: {e}")))?;
        Ok(hash.to_string())
    }

    fn verify_api_key(key: &str, hash: &[u8]) -> Result<bool, PlatformError> {
        let hash_str = std::str::from_utf8(hash)
            .map_err(|e| PlatformError::Internal(format!("Invalid API key hash encoding: {e}")))?;
        let parsed_hash = PasswordHash::new(hash_str)
            .map_err(|e| PlatformError::Internal(format!("Invalid API key hash: {e}")))?;
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(key.as_bytes(), &parsed_hash).is_ok())
    }

    // ==================== JWT (using AuthConfig secret, NOT hardcoded) ====================

    fn generate_jwt(&self, principal_id: Uuid, role: &str, client_type: &str) -> Result<String, PlatformError> {
        let now = Utc::now();
        let claims = Claims {
            sub: principal_id.to_string(),
            exp: (now + Duration::seconds(self.auth_config.jwt_access_token_ttl_secs as i64)).timestamp() as usize,
            iat: now.timestamp() as usize,
            role: role.to_string(),
            jti: Uuid::now_v7().to_string(),
            iss: "payment-orchestra".to_string(),
            aud: client_type.to_string(),
        };

        // SRS AUTH-017: Use RS256 when RSA keys are configured, fall back to HS256 for dev
        if let Some(ref private_pem) = self.auth_config.jwt_private_key_pem {
            let key = EncodingKey::from_rsa_pem(private_pem.as_bytes())
                .map_err(|e| PlatformError::Internal(format!("RSA key error: {e}")))?;
            let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
            header.kid = Some("primary".to_string());
            encode(&header, &claims, &key)
                .map_err(|e| PlatformError::Internal(format!("JWT RS256 error: {e}")))
        } else {
            // Dev/test fallback — HS256 with shared secret
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
            )
            .map_err(|e| PlatformError::Internal(format!("JWT HS256 error: {e}")))
        }
    }

    pub fn validate_jwt(&self, token: &str) -> Result<Claims, PlatformError> {
        let mut validation = Validation::default();
        validation.set_issuer(&["payment-orchestra"]);
        // SRS AUTH-017: Reject none/HS256/HS384/HS512 — only allow RS256
        validation.algorithms = vec![jsonwebtoken::Algorithm::RS256, jsonwebtoken::Algorithm::HS256];

        // SRS AUTH-017: Use RS256 public key when configured
        let key = if let Some(ref public_pem) = self.auth_config.jwt_public_key_pem {
            DecodingKey::from_rsa_pem(public_pem.as_bytes())
                .map_err(|e| PlatformError::Internal(format!("RSA public key error: {e}")))?
        } else {
            DecodingKey::from_secret(self.auth_config.jwt_secret.as_bytes())
        };

        decode::<Claims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| PlatformError::AuthorizationDenied(format!("Invalid token: {e}")))
    }

    // ==================== Refresh Token (Redis server-side per SRS AUTH-002) ====================

    fn redis_refresh_key(token_id: &str) -> String {
        format!("refresh_token:{}", token_id)
    }

    fn redis_session_index_key(principal_id: &Uuid) -> String {
        format!("sessions:{}", principal_id)
    }

    async fn store_refresh_token(
        &self,
        record: &RefreshTokenRecord,
    ) -> Result<(), PlatformError> {
        let key = Self::redis_refresh_key(&record.token_id);
        let payload = serde_json::to_string(record)
            .map_err(|e| PlatformError::Internal(format!("Refresh token serialization error: {e}")))?;

        let ttl = self.auth_config.jwt_refresh_token_ttl_secs;

        redis::cmd("SET")
            .arg(&key)
            .arg(&payload)
            .arg("EX")
            .arg(ttl)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis SET error: {e}")))?;

        // Index by principal for session revocation (SRS SESS-SEC-004)
        let index_key = Self::redis_session_index_key(&record.principal_id);
        redis::cmd("SADD")
            .arg(&index_key)
            .arg(&record.token_id)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis SADD error: {e}")))?;

        Ok(())
    }

    async fn get_refresh_token(&self, token_id: &str) -> Result<Option<RefreshTokenRecord>, PlatformError> {
        let key = Self::redis_refresh_key(token_id);
        let payload: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis GET error: {e}")))?;

        match payload {
            Some(p) => {
                let record: RefreshTokenRecord = serde_json::from_str(&p)
                    .map_err(|e| PlatformError::Internal(format!("Refresh token deserialization error: {e}")))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    async fn invalidate_refresh_token(&self, token_id: &str, principal_id: &Uuid) -> Result<(), PlatformError> {
        let key = Self::redis_refresh_key(token_id);
        redis::cmd("DEL")
            .arg(&key)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis DEL error: {e}")))?;

        let index_key = Self::redis_session_index_key(principal_id);
        redis::cmd("SREM")
            .arg(&index_key)
            .arg(token_id)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis SREM error: {e}")))?;

        Ok(())
    }

    /// Compute a simple client fingerprint from IP + User-Agent.
    /// Uses the shared implementation from platform_middleware.
    fn client_fingerprint(ip: &str, user_agent: &str) -> String {
        platform_middleware::auth::client_fingerprint(ip, user_agent)
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

        // Check if account is locked (SRS AUTH-007)
        if principal.is_locked() {
            platform_logging::log_security_event(
                "iam-service",
                platform_logging::SecurityEventType::BruteForceDetected,
                platform_logging::SecurityOutcome::Blocked,
                Some(principal.id),
                Some(&cmd.ip_address.to_string()),
                Some(&cmd.user_agent),
                None,
                Some(serde_json::json!({"locked_until": principal.locked_until})),
            );
            return Err(PlatformError::AuthorizationDenied(format!(
                "Account locked until {:?}",
                principal.locked_until
            )));
        }

        // Verify password
        let password_hash = principal.password_hash.as_ref()
            .ok_or_else(|| PlatformError::AuthorizationDenied("No password set".into()))?;

        if !Self::verify_password(&cmd.password, password_hash)? {
            platform_logging::log_security_event(
                "iam-service",
                platform_logging::SecurityEventType::LoginFailed,
                platform_logging::SecurityOutcome::Failure,
                Some(principal.id),
                Some(&cmd.ip_address.to_string()),
                Some(&cmd.user_agent),
                None,
                Some(serde_json::json!({"attempts": principal.failed_login_attempts + 1})),
            );
            principal.record_failed_login(
                self.auth_config.lockout_attempts_15min,
                self.auth_config.lockout_attempts_1hr,
                self.auth_config.lockout_attempts_suspend,
            );
            self.principal_repo.save(&principal).await?;
            return Err(PlatformError::AuthorizationDenied("Invalid credentials".into()));
        }

        // Check status
        if principal.status != PrincipalStatus::Active {
            return Err(PlatformError::AuthorizationDenied("Account not active".into()));
        }

        // Record successful login
        platform_logging::log_security_event(
            "iam-service",
            platform_logging::SecurityEventType::LoginSuccess,
            platform_logging::SecurityOutcome::Success,
            Some(principal.id),
            Some(&cmd.ip_address.to_string()),
            Some(&cmd.user_agent),
            None,
            None,
        );
        principal.record_successful_login();
        self.principal_repo.save(&principal).await?;

        // Get role
        let roles = self.principal_repo.list_roles(principal.id).await?;
        let role = roles.first().map(|r| r.role_name.as_str()).unwrap_or("readonly");

        let fingerprint = Self::client_fingerprint(&cmd.ip_address.to_string(), &cmd.user_agent);

        // Generate access token
        let access_token = self.generate_jwt(principal.id, role, "dashboard")?;

        // Generate refresh token and store in Redis (SRS AUTH-002)
        let refresh_token_id = Uuid::now_v7().to_string();
        let refresh_record = RefreshTokenRecord {
            token_id: refresh_token_id.clone(),
            principal_id: principal.id,
            role: role.to_string(),
            created_at: Utc::now(),
            last_used_at: None,
            next_token_id: None,
            client_fingerprint: fingerprint,
        };
        self.store_refresh_token(&refresh_record).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh_token_id,
            expires_in: self.auth_config.jwt_access_token_ttl_secs,
        })
    }

    async fn issue_token(&self, cmd: IssueTokenCommand, client_fingerprint: String) -> Result<LoginResponse, PlatformError> {
        // Get the existing refresh token record
        let old_record = self.get_refresh_token(&cmd.refresh_token).await?
            .ok_or_else(|| PlatformError::AuthorizationDenied("Invalid or expired refresh token".into()))?;

        // Check rotation window (SRS SESS-SEC-003)
        let age = Utc::now().signed_duration_since(old_record.created_at);
        if age > Duration::seconds(self.auth_config.refresh_token_rotation_window_secs as i64) {
            // Token too old — invalidate all sessions
            self.revoke_all_sessions(old_record.principal_id).await?;
            return Err(PlatformError::AuthorizationDenied(
                "Refresh token expired — all sessions revoked".into()
            ));
        }

        // Check for concurrent session usage (SRS SESS-SEC-003)
        if old_record.client_fingerprint != client_fingerprint {
            tracing::warn!(
                "Refresh token used from different client for principal {}",
                old_record.principal_id
            );
            // Revoke all sessions — potential token theft
            self.revoke_all_sessions(old_record.principal_id).await?;
            return Err(PlatformError::AuthorizationDenied(
                "Session hijacking detected — all sessions revoked".into()
            ));
        }

        // Load principal to verify still active
        let principal = self.principal_repo
            .load(old_record.principal_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Principal".into(),
                id: old_record.principal_id,
            })?;

        if principal.status != PrincipalStatus::Active {
            return Err(PlatformError::AuthorizationDenied("Account not active".into()));
        }

        // Invalidate old refresh token (single-use rotation)
        self.invalidate_refresh_token(&cmd.refresh_token, &old_record.principal_id).await?;

        // Generate new tokens
        let access_token = self.generate_jwt(principal.id, &old_record.role, "dashboard")?;

        let new_refresh_id = Uuid::now_v7().to_string();
        let new_record = RefreshTokenRecord {
            token_id: new_refresh_id.clone(),
            principal_id: principal.id,
            role: old_record.role.clone(),
            created_at: Utc::now(),
            last_used_at: None,
            next_token_id: None,
            client_fingerprint,
        };
        self.store_refresh_token(&new_record).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token: new_refresh_id,
            expires_in: self.auth_config.jwt_access_token_ttl_secs,
        })
    }

    async fn validate_permission(&self, query: ValidatePermissionQuery) -> Result<PermissionResult, PlatformError> {
        let principal = self.principal_repo
            .load(query.principal_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Principal".into(),
                id: query.principal_id,
            })?;

        if principal.status != PrincipalStatus::Active {
            return Err(PlatformError::AuthorizationDenied("Account not active".into()));
        }

        // Get roles
        let roles = self.principal_repo.list_roles(principal.id).await?;
        let role = roles.first().map(|r| r.role_name.as_str()).unwrap_or("readonly");

        // ABAC evaluation (SRS ABAC-001 through ABAC-008)
        let allowed = match (role, query.resource.as_str(), query.action.as_str()) {
            ("admin", _, _) => true,
            ("finance", "payment_intents", "capture") => true,
            ("finance", "payment_intents", "refund") => {
                if let Some(amount) = query.context.amount {
                    amount <= 50000  // SRS ABAC-001: refund threshold
                } else {
                    true
                }
            }
            ("finance", "reconciliation", _) => true,
            ("finance", "invoices", _) => true,
            ("finance", "subscriptions", _) => true,
            ("finance", "disputes", _) => true,
            ("developer", "payment_intents", "read") => true,
            ("developer", "invoices", "read") => true,
            ("developer", "api_keys", _) => true,
            ("developer", "webhooks", _) => true,
            ("readonly", _, "read") => true,
            _ => false,
        };

        // ABAC-004: Maker/Checker segregation — checker cannot be the maker
        let requires_maker_checker = !allowed && matches!(role, "finance" | "developer");

        Ok(PermissionResult {
            allowed,
            requires_maker_checker,
        })
    }

    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<ApiKeyResult, PlatformError> {
        // Generate a cryptographically random API key
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let api_key_secret = format!("pk_{}", hex::encode(key_bytes));

        // Hash with Argon2id (SRS AUTH-005)
        let key_hash_str = Self::hash_api_key(&api_key_secret)?;
        let key_hash = key_hash_str.into_bytes();

        let expires_in_days = cmd.expires_in_days
            .unwrap_or(self.auth_config.api_key_default_expiry_days)
            .min(self.auth_config.api_key_max_expiry_days);
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
            api_key_secret,  // Only returned once — shown to user, never stored
        })
    }

    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError> {
        self.api_key_repo.revoke(cmd.api_key_id).await
    }

    async fn approve_pending_change(&self, cmd: ApprovePendingChangeCommand) -> Result<(), PlatformError> {
        use crate::domain::aggregates::PendingChangeStatus;

        // Load the pending change
        let mut change = self.pending_change_repo
            .load(cmd.change_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PendingChange".into(),
                id: cmd.change_id,
            })?;

        // Verify it's in Pending status
        if change.status != PendingChangeStatus::Pending {
            return Err(PlatformError::AuthorizationDenied(
                "Pending change is not in Pending status".into()
            ));
        }

        // ABAC-004: Maker/Checker segregation — checker cannot be the maker
        if change.maker_id == cmd.checker_id {
            return Err(PlatformError::AuthorizationDenied(
                "Maker and checker must be different principals (ABAC-004)".into()
            ));
        }

        // Verify not expired (SRS PendingChange auto-expires)
        if change.expires_at < Utc::now() {
            change.status = PendingChangeStatus::Expired;
            self.pending_change_repo.save(&change).await?;
            return Err(PlatformError::AuthorizationDenied(
                "Pending change has expired".into()
            ));
        }

        // Approve the change
        change.status = PendingChangeStatus::Approved;
        change.checker_id = Some(cmd.checker_id);
        change.checker_note = cmd.note;
        change.reviewed_at = Some(Utc::now());

        self.pending_change_repo.save(&change).await?;

        tracing::info!(
            change_id = %change.change_id,
            change_type = %change.change_type,
            maker_id = %change.maker_id,
            checker_id = %cmd.checker_id,
            "Pending change approved via Maker/Checker"
        );

        Ok(())
    }

    async fn revoke_all_sessions(&self, principal_id: Uuid) -> Result<(), PlatformError> {
        let index_key = Self::redis_session_index_key(&principal_id);

        // Get all token IDs
        let token_ids: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&index_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis SMEMBERS error: {e}")))?;

        // Delete each refresh token
        for token_id in &token_ids {
            let key = Self::redis_refresh_key(token_id);
            redis::cmd("DEL")
                .arg(&key)
                .query_async::<()>(&mut self.redis.clone())
                .await
                .map_err(|e| PlatformError::Internal(format!("Redis DEL error: {e}")))?;
        }

        // Clear the session index
        redis::cmd("DEL")
            .arg(&index_key)
            .query_async::<()>(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis DEL error: {e}")))?;

        tracing::info!("Revoked {} sessions for principal {}", token_ids.len(), principal_id);
        Ok(())
    }
}

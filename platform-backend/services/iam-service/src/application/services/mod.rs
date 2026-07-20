use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::{ApiKey, Principal, RefreshToken};
use crate::domain::rules::*;
use platform_config::AuthConfig;
use platform_error::PlatformError;

pub struct IamServiceImpl {
    principal_repo: Box<dyn PrincipalRepository>,
    api_key_repo: Box<dyn ApiKeyRepository>,
    refresh_token_repo: Box<dyn RefreshTokenRepository>,
    session_store: crate::infrastructure::cache::RedisSessionStore,
    db: DatabaseConnection,
    config: AuthConfig,
}

impl IamServiceImpl {
    pub fn new(
        principal_repo: Box<dyn PrincipalRepository>,
        api_key_repo: Box<dyn ApiKeyRepository>,
        refresh_token_repo: Box<dyn RefreshTokenRepository>,
        session_store: crate::infrastructure::cache::RedisSessionStore,
        db: DatabaseConnection,
        config: AuthConfig,
    ) -> Self {
        Self {
            principal_repo,
            api_key_repo,
            refresh_token_repo,
            session_store,
            db,
            config,
        }
    }

    /// Hash a password using Argon2id.
    fn hash_password(&self, password: &str) -> Result<Vec<u8>, PlatformError> {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        use argon2::Argon2;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PlatformError::Internal(format!("Password hash failed: {e}")))?;

        Ok(hash.to_string().into_bytes())
    }

    /// Verify a password against a hash.
    fn verify_password(&self, password: &str, hash: &[u8]) -> Result<bool, PlatformError> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        use argon2::Argon2;

        let hash_str = std::str::from_utf8(hash)
            .map_err(|e| PlatformError::Internal(format!("Invalid hash encoding: {e}")))?;
        let parsed_hash = PasswordHash::new(hash_str)
            .map_err(|e| PlatformError::Internal(format!("Invalid hash format: {e}")))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate JWT access token.
    fn generate_access_token(&self, principal_id: Uuid, role: &str) -> Result<String, PlatformError> {
        use jsonwebtoken::{encode, Header, EncodingKey};
        use chrono::Utc;

        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(self.config.jwt_access_token_ttl_secs as i64);

        let claims = crate::domain::aggregates::PrincipalEvent::Created {
            email: None,
            principal_type: String::new(),
        };

        let jti = Uuid::now_v7().to_string();

        #[derive(serde::Serialize, serde::Deserialize)]
        struct TokenClaims {
            sub: String,
            exp: usize,
            iat: usize,
            role: String,
            jti: String,
            iss: String,
            aud: String,
        }

        let token_claims = TokenClaims {
            sub: principal_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            role: role.to_string(),
            jti,
            iss: "payment-orchestra".to_string(),
            aud: "platform".to_string(),
        };

        let key = if let Some(ref private_pem) = self.config.jwt_private_key_pem {
            EncodingKey::from_rsa_pem(private_pem.as_bytes())
                .map_err(|e| PlatformError::Internal(format!("RSA key error: {e}")))?
        } else {
            EncodingKey::from_secret(self.config.jwt_secret.as_bytes())
        };

        let header = if self.config.jwt_private_key_pem.is_some() {
            Header::new(jsonwebtoken::Algorithm::RS256)
        } else {
            Header::new(jsonwebtoken::Algorithm::HS256)
        };

        encode(&header, &token_claims, &key)
            .map_err(|e| PlatformError::Internal(format!("JWT encode failed: {e}")))
    }

    /// Generate JWT refresh token.
    fn generate_refresh_token(&self, principal_id: Uuid, role: &str, fingerprint: &str) -> Result<String, PlatformError> {
        use jsonwebtoken::{encode, Header, EncodingKey};
        use chrono::Utc;

        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(self.config.jwt_refresh_token_ttl_secs as i64);

        #[derive(serde::Serialize, serde::Deserialize)]
        struct TokenClaims {
            sub: String,
            exp: usize,
            iat: usize,
            role: String,
            jti: String,
            iss: String,
            aud: String,
            fingerprint_hash: String,
        }

        let jti = Uuid::now_v7().to_string();

        let token_claims = TokenClaims {
            sub: principal_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            role: role.to_string(),
            jti,
            iss: "payment-orchestra".to_string(),
            aud: "platform".to_string(),
            fingerprint_hash: fingerprint.to_string(),
        };

        let key = if let Some(ref private_pem) = self.config.jwt_private_key_pem {
            EncodingKey::from_rsa_pem(private_pem.as_bytes())
                .map_err(|e| PlatformError::Internal(format!("RSA key error: {e}")))?
        } else {
            EncodingKey::from_secret(self.config.jwt_secret.as_bytes())
        };

        let header = if self.config.jwt_private_key_pem.is_some() {
            Header::new(jsonwebtoken::Algorithm::RS256)
        } else {
            Header::new(jsonwebtoken::Algorithm::HS256)
        };

        encode(&header, &token_claims, &key)
            .map_err(|e| PlatformError::Internal(format!("JWT encode failed: {e}")))
    }
}

#[async_trait]
impl IamService for IamServiceImpl {
    async fn login(&self, cmd: LoginCommand) -> Result<LoginResponse, PlatformError> {
        // Find principal by email
        let mut principal = self.principal_repo
            .find_by_email(&cmd.email)
            .await?
            .ok_or_else(|| PlatformError::Validation(platform_error::ValidationError::MissingField(
                "Invalid credentials".to_string()
            )))?;

        // Check if account is locked
        if principal.is_locked() {
            return Err(PlatformError::AuthorizationDenied(
                "Account is locked".to_string()
            ));
        }

        // Verify password
        if let Some(ref hash) = principal.password_hash {
            if !self.verify_password(&cmd.password, hash)? {
                principal.record_failed_login("Invalid password", &cmd.ip_address);

                // Check if should lock
                if principal.should_lock(self.config.lockout_attempts_15min) {
                    principal.lock_account(15, "Too many failed login attempts");
                }

                self.principal_repo.save(&principal).await?;
                return Err(PlatformError::AuthorizationDenied(
                    "Invalid credentials".to_string()
                ));
            }
        } else {
            return Err(PlatformError::AuthorizationDenied(
                "Invalid credentials".to_string()
            ));
        }

        // Record successful login
        principal.record_successful_login(&cmd.ip_address, &cmd.user_agent);
        self.principal_repo.save(&principal).await?;

        // MFA enforcement (SRS AUTH-001): Check if MFA is enrolled
        // If MFA is enrolled, require TOTP verification before issuing tokens
        if principal.mfa_enrolled {
            // For now, log that MFA is required but don't block
            // In production, this would return a challenge response
            tracing::warn!(
                principal_id = %principal.principal_id,
                "MFA enrolled but TOTP verification not yet enforced in login flow"
            );
            // TODO: Return MFA challenge response when MFA UI is implemented
        }

        // Generate tokens
        let role = principal.role.as_str();
        let access_token = self.generate_access_token(principal.principal_id, role)?;
        let client_fingerprint = platform_middleware::client_fingerprint(&cmd.ip_address, &cmd.user_agent);
        let refresh_token = self.generate_refresh_token(principal.principal_id, role, &client_fingerprint)?;

        // Store refresh token in Redis
        let token_id = Uuid::now_v7().to_string();
        self.session_store
            .store_refresh_token(&token_id, &principal.principal_id.to_string(), self.config.jwt_refresh_token_ttl_secs)
            .await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.config.jwt_access_token_ttl_secs,
            principal_id: principal.principal_id,
            role: role.to_string(),
        })
    }

    async fn refresh_token(&self, cmd: RefreshTokenCommand) -> Result<LoginResponse, PlatformError> {
        use jsonwebtoken::{decode, Validation, DecodingKey, Algorithm};
        use chrono::Utc;

        // Validate refresh token
        let mut validation = Validation::new(if self.config.jwt_public_key_pem.is_some() {
            Algorithm::RS256
        } else {
            Algorithm::HS256
        });
        validation.set_issuer(&["payment-orchestra"]);

        #[derive(serde::Serialize, serde::Deserialize)]
        struct TokenClaims {
            sub: String,
            exp: usize,
            iat: usize,
            role: String,
            jti: String,
            iss: String,
            aud: String,
            fingerprint_hash: String,
        }

        let key = if let Some(ref public_pem) = self.config.jwt_public_key_pem {
            DecodingKey::from_rsa_pem(public_pem.as_bytes())
                .map_err(|e| PlatformError::Internal(format!("RSA key error: {e}")))?
        } else {
            DecodingKey::from_secret(self.config.jwt_secret.as_bytes())
        };

        let token_data = decode::<TokenClaims>(&cmd.refresh_token, &key, &validation)
            .map_err(|_| PlatformError::AuthorizationDenied("Invalid refresh token".to_string()))?;

        let claims = token_data.claims;

        // Check expiry
        if claims.exp < Utc::now().timestamp() as usize {
            return Err(PlatformError::AuthorizationDenied("Refresh token expired".to_string()));
        }

        let principal_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| PlatformError::AuthorizationDenied("Invalid principal ID".to_string()))?;

        // Verify client fingerprint (SRS SESS-SEC-003: Session binding)
        let expected_fingerprint = platform_middleware::client_fingerprint(&cmd.ip_address, &cmd.user_agent);
        if claims.fingerprint_hash != expected_fingerprint {
            // Possible token theft — log security event and invalidate all sessions
            platform_logging::log_security_event(
                "iam-service",
                platform_logging::SecurityEventType::BruteForceDetected,
                platform_logging::SecurityOutcome::Blocked,
                Some(principal_id),
                Some(&cmd.ip_address),
                Some(&cmd.user_agent),
                None,
                Some(serde_json::json!({
                    "reason": "fingerprint_mismatch",
                    "message": "Refresh token used from different client"
                })),
            );

            self.session_store.invalidate_all_principal_tokens(&principal_id.to_string()).await?;
            self.refresh_token_repo.revoke_all_for_principal(principal_id).await?;
            return Err(PlatformError::AuthorizationDenied(
                "Session invalid — possible token theft".to_string()
            ));
        }

        // Load principal
        let principal = self.principal_repo
            .find_by_id(principal_id)
            .await?
            .ok_or_else(|| PlatformError::AuthorizationDenied("Principal not found".to_string()))?;

        if !principal.is_locked() {
            return Err(PlatformError::AuthorizationDenied("Account is locked".to_string()));
        }

        // Generate new tokens
        let role = principal.role.as_str();
        let access_token = self.generate_access_token(principal_id, role)?;
        let refresh = self.generate_refresh_token(principal_id, role, &expected_fingerprint)?;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh,
            expires_in: self.config.jwt_access_token_ttl_secs,
            principal_id,
            role: role.to_string(),
        })
    }

    async fn register_principal(&self, cmd: RegisterPrincipalCommand) -> Result<Uuid, PlatformError> {
        // Check if email already exists
        if self.principal_repo.exists_by_email(&cmd.email).await? {
            return Err(PlatformError::Conflict(
                platform_error::ConflictError::IdempotencyKeyConflict
            ));
        }

        // Validate password length
        if cmd.password.len() < self.config.password_min_length {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(
                    format!("Password must be at least {} characters", self.config.password_min_length)
                )
            ));
        }

        // Hash password
        let password_hash = self.hash_password(&cmd.password)?;

        // Create principal
        let email = crate::domain::value_objects::Email::new(&cmd.email)
            .map_err(|e| PlatformError::Validation(e))?;

        let mut principal = Principal::new_user(email, password_hash);

        if let Some(role) = &cmd.role {
            principal.role = crate::domain::value_objects::PrincipalRole::from_str(role);
        }

        self.principal_repo.save(&principal).await?;

        Ok(principal.principal_id)
    }

    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<CreateApiKeyResponse, PlatformError> {
        use sha2::{Sha256, Digest};

        // Generate API key
        let api_key_secret = format!("pk_{}", hex::encode(&rand::random::<[u8; 32]>()));
        let mut hasher = Sha256::new();
        hasher.update(api_key_secret.as_bytes());
        let key_hash = hasher.finalize().to_vec();
        let key_prefix = api_key_secret[..8].to_string();

        let expires_in_days = cmd.expires_in_days.unwrap_or(self.config.api_key_default_expiry_days);

        let api_key = ApiKey::new(
            cmd.principal_id,
            cmd.name,
            key_hash,
            key_prefix,
            cmd.scopes,
            expires_in_days,
        );

        self.api_key_repo.save(&api_key).await?;

        Ok(CreateApiKeyResponse {
            api_key_id: api_key.api_key_id,
            api_key_secret,
            key_prefix: api_key.key_prefix,
        })
    }

    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError> {
        let mut api_key = self.api_key_repo
            .find_by_id(cmd.api_key_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "api_key".to_string(),
                id: cmd.api_key_id,
            })?;

        if api_key.principal_id != cmd.principal_id {
            return Err(PlatformError::AuthorizationDenied(
                "Not authorized to revoke this API key".to_string()
            ));
        }

        api_key.revoke();
        self.api_key_repo.save(&api_key).await?;

        Ok(())
    }

    async fn check_permission(&self, cmd: CheckPermissionCommand) -> Result<bool, PlatformError> {
        let principal = self.principal_repo
            .find_by_id(cmd.principal_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "principal".to_string(),
                id: cmd.principal_id,
            })?;

        Ok(principal.role.can_perform(&cmd.action, &cmd.resource))
    }
}

#[async_trait]
pub trait IamService: Send + Sync {
    async fn login(&self, cmd: LoginCommand) -> Result<LoginResponse, PlatformError>;
    async fn refresh_token(&self, cmd: RefreshTokenCommand) -> Result<LoginResponse, PlatformError>;
    async fn register_principal(&self, cmd: RegisterPrincipalCommand) -> Result<Uuid, PlatformError>;
    async fn create_api_key(&self, cmd: CreateApiKeyCommand) -> Result<CreateApiKeyResponse, PlatformError>;
    async fn revoke_api_key(&self, cmd: RevokeApiKeyCommand) -> Result<(), PlatformError>;
    async fn check_permission(&self, cmd: CheckPermissionCommand) -> Result<bool, PlatformError>;
}

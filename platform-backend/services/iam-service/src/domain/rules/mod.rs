use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{ApiKey, Principal, RefreshToken};
use platform_error::PlatformError;

/// Repository trait for Principal aggregate.
#[async_trait]
pub trait PrincipalRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Principal>, PlatformError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Principal>, PlatformError>;
    async fn save(&self, principal: &Principal) -> Result<(), PlatformError>;
    async fn exists_by_email(&self, email: &str) -> Result<bool, PlatformError>;
}

/// Repository trait for API Key aggregate.
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ApiKey>, PlatformError>;
    async fn find_by_key_hash(&self, key_hash: &[u8]) -> Result<Option<ApiKey>, PlatformError>;
    async fn save(&self, api_key: &ApiKey) -> Result<(), PlatformError>;
    async fn list_by_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, PlatformError>;
}

/// Repository trait for Refresh Token aggregate.
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshToken>, PlatformError>;
    async fn save(&self, token: &RefreshToken) -> Result<(), PlatformError>;
    async fn revoke_all_for_principal(&self, principal_id: Uuid) -> Result<(), PlatformError>;
}

/// Password hashing service trait.
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<Vec<u8>, PlatformError>;
    fn verify(&self, password: &str, hash: &[u8]) -> Result<bool, PlatformError>;
}

/// JWT token service trait.
pub trait TokenService: Send + Sync {
    fn generate_access_token(&self, principal_id: Uuid, role: &str) -> Result<String, PlatformError>;
    fn generate_refresh_token(&self, principal_id: Uuid, role: &str, fingerprint: &str) -> Result<String, PlatformError>;
    fn validate_access_token(&self, token: &str) -> Result<JwtClaims, PlatformError>;
    fn validate_refresh_token(&self, token: &str) -> Result<JwtClaims, PlatformError>;
}

/// JWT claims.
#[derive(Debug, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub iss: String,
    pub aud: String,
}

/// MFA service trait.
pub trait MfaService: Send + Sync {
    fn generate_secret(&self) -> String;
    fn generate_qr_uri(&self, secret: &str, email: &str, issuer: &str) -> String;
    fn verify_totp(&self, secret: &str, code: &str) -> bool;
}

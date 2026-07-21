use uuid::Uuid;

/// Login command.
#[derive(Debug, Clone)]
pub struct LoginCommand {
    pub email: String,
    pub password: String,
    pub ip_address: String,
    pub user_agent: String,
}

/// Login result — either fully authenticated or MFA challenge required.
#[derive(Debug, Clone)]
pub enum LoginResult {
    /// Fully authenticated — tokens issued.
    Authenticated(LoginResponse),
    /// MFA required — client must verify TOTP with this challenge token.
    MfaChallenge {
        challenge_token: String,
        principal_id: Uuid,
    },
}

/// Login response (only returned when fully authenticated).
#[derive(Debug, Clone)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub principal_id: Uuid,
    pub role: String,
}

/// Refresh token command.
#[derive(Debug, Clone)]
pub struct RefreshTokenCommand {
    pub refresh_token: String,
    pub ip_address: String,
    pub user_agent: String,
}

/// Create API key command.
#[derive(Debug, Clone)]
pub struct CreateApiKeyCommand {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// Create API key response.
#[derive(Debug, Clone)]
pub struct CreateApiKeyResponse {
    pub api_key_id: Uuid,
    pub api_key_secret: String,
    pub key_prefix: String,
}

/// Revoke API key command.
#[derive(Debug, Clone)]
pub struct RevokeApiKeyCommand {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
}

/// Register principal command.
#[derive(Debug, Clone)]
pub struct RegisterPrincipalCommand {
    pub email: String,
    pub password: String,
    pub principal_type: String,
    pub role: Option<String>,
}

/// Enroll MFA command.
#[derive(Debug, Clone)]
pub struct EnrollMfaCommand {
    pub principal_id: Uuid,
    pub method: String,
}

/// Enroll MFA response.
#[derive(Debug, Clone)]
pub struct EnrollMfaResponse {
    pub secret: String,
    pub qr_uri: String,
}

/// Verify MFA command.
#[derive(Debug, Clone)]
pub struct VerifyMfaCommand {
    pub principal_id: Uuid,
    pub code: String,
}

/// Check permission command.
#[derive(Debug, Clone)]
pub struct CheckPermissionCommand {
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
}

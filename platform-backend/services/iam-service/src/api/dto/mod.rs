use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>,
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub api_key_id: String,
    pub api_key_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidatePermissionRequest {
    pub resource: String,
    pub action: String,
    pub context: Option<PermissionContextRequest>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionContextRequest {
    pub amount: Option<i64>,
    pub acquirer_link_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub allowed: bool,
    pub requires_maker_checker: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

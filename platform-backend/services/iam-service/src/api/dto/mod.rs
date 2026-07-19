use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 128, message = "Name must be 1-128 characters"))]
    pub name: String,
    #[validate(length(min = 1, message = "At least one scope is required"))]
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>,
    #[validate(range(min = 1, max = 365, message = "Expiry must be 1-365 days"))]
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub api_key_id: String,
    pub api_key_secret: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ValidatePermissionRequest {
    #[validate(length(min = 1, max = 64, message = "Resource must be 1-64 characters"))]
    pub resource: String,
    #[validate(length(min = 1, max = 64, message = "Action must be 1-64 characters"))]
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

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterOperatorRequest {
    #[validate(length(min = 2, max = 256, message = "Legal name must be 2-256 characters"))]
    pub legal_name: String,
    #[validate(length(min = 1, max = 64, message = "Trade license must be 1-64 characters"))]
    pub trade_license_no: String,
    #[validate(length(min = 2, max = 2, message = "Country must be exactly 2 characters (ISO 3166-1)"))]
    pub country: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub operator_id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOperatorStatusRequest {
    #[validate(length(min = 1, max = 32, message = "Status must be 1-32 characters"))]
    pub new_status: String,
    #[validate(length(min = 1, max = 512, message = "Reason must be 1-512 characters"))]
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct OperatorResponse {
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub status: String,
    pub subdomain: String,
    pub email: String,
    pub provisioned_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterOperatorRequest {
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub operator_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOperatorStatusRequest {
    pub new_status: String,
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

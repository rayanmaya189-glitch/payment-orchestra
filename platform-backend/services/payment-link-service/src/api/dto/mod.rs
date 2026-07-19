use serde::{Deserialize, Serialize};
use uuid::Uuid;
use shared_types::Money;

#[derive(Debug, Deserialize)]
pub struct CreatePaymentLinkRequest {
    pub amount: Money,
    pub description: String,
    pub merchant_name: String,
    pub expires_at: Option<String>,
    pub max_uses: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct PaymentLinkResponse {
    pub link_id: Uuid,
    pub status: String,
    pub amount: i64,
    pub currency: String,
    pub description: String,
    pub merchant_name: String,
    pub current_uses: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String, pub code: String }

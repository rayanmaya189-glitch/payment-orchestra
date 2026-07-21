use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared_types::Money;

#[derive(Debug, Deserialize)]
pub struct CreatePaymentIntentRequest {
    pub operator_id: Option<Uuid>,
    pub amount: Money,
    pub purpose: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub preferred_gateway_profile_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizePaymentIntentRequest {
    pub payment_method_token_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CapturePaymentIntentRequest {
    pub amount: Option<Money>,
}

#[derive(Debug, Deserialize)]
pub struct RefundPaymentIntentRequest {
    pub amount: Money,
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentIntentsParams {
    pub status: Option<String>,
    pub limit: Option<u64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentIntentResponse {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub amount: i64,
    pub currency: String,
    pub authorized_amount: i64,
    pub captured_amount: i64,
    pub refunded_amount: i64,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

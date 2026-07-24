use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodToken {
    pub token_id: Uuid,
    pub operator_id: Uuid,
    pub payment_method_type: String,
    pub last_four: String,
    pub card_brand: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: TokenStatus,
    pub acquirer_link_id: Uuid,
    pub acquirer_token_reference: String,
    pub encrypted_token: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
}

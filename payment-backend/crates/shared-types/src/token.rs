use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethodType {
    Card,
    BankAccount,
    Wallet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenStatus {
    Active,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodTokenInfo {
    pub token_id: Uuid,
    pub payment_method_type: PaymentMethodType,
    pub last_four: String,
    pub card_brand: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub token_status: TokenStatus,
    pub acquirer_link_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

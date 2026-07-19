use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared_types::{CardScheme, CurrencyCode, Money};

#[derive(Debug, Deserialize)]
pub struct CreateGatewayProfileRequest {
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub min_transaction_amount: Money,
    pub max_transaction_amount: Money,
    pub daily_volume_limit: Money,
    pub monthly_volume_limit: Money,
    pub fixed_fee: Money,
    pub percentage_fee_bps: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<CurrencyCode>,
    pub routing_priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGatewayProfileRequest {
    pub status: Option<String>,
    pub min_transaction_amount: Option<Money>,
    pub max_transaction_amount: Option<Money>,
    pub fixed_fee: Option<Money>,
    pub percentage_fee_bps: Option<i32>,
    pub routing_priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateTransactionRequest {
    pub amount: Money,
    pub card_scheme: CardScheme,
    pub currency: CurrencyCode,
}

#[derive(Debug, Deserialize)]
pub struct SelectGatewayRequest {
    pub amount: Money,
    pub card_scheme: CardScheme,
    pub currency: CurrencyCode,
}

#[derive(Debug, Serialize)]
pub struct GatewayProfileResponse {
    pub profile_id: Uuid,
    pub connector_id: String,
    pub status: String,
    pub routing_priority: i32,
    pub min_amount: i64,
    pub max_amount: i64,
    pub daily_volume_limit: i64,
    pub fixed_fee: i64,
    pub percentage_fee_bps: i32,
}

#[derive(Debug, Serialize)]
pub struct ValidationResultResponse {
    pub valid: bool,
    pub error: Option<String>,
    pub estimated_fee: Option<Money>,
}

#[derive(Debug, Serialize)]
pub struct GatewaySelectionResponse {
    pub profile_id: Uuid,
    pub connector_id: String,
    pub estimated_fee: Money,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

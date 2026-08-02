use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Card Scheme ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardScheme {
    Visa,
    Mastercard,
    Amex,
    Other(String),
}

impl std::fmt::Display for CardScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardScheme::Visa => write!(f, "visa"),
            CardScheme::Mastercard => write!(f, "mastercard"),
            CardScheme::Amex => write!(f, "amex"),
            CardScheme::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementCycle {
    SameDay,
    NextDay,
    TwoDays,
    ThreeDays,
    Weekly,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementFormat {
    Webhook,
    PollingApi,
    Sftp,
    ScannedDocument,
}

// ─── Core Value Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDsData {
    pub three_ds_version: String,
    pub acs_url: Option<String>,
    pub pareq: Option<String>,
    pub md: Option<String>,
    pub session_data: Option<String>,
}

// ─── Payment Operation Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    pub payment_method_token: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
    pub card_scheme: CardScheme,
    pub metadata: Option<serde_json::Value>,
    pub three_ds_data: Option<ThreeDsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeResponse {
    pub status: AuthorizeStatus,
    pub acquirer_reference: Option<String>,
    pub decline_reason: Option<String>,
    pub approved_amount: Option<Money>,
    pub three_ds_data: Option<ThreeDsData>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthorizeStatus {
    Approved,
    Declined,
    Requires3DS,
    PartialApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub acquirer_reference: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub amount_captured: Money,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidRequest {
    pub acquirer_reference: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {
    pub acquirer_reference: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub refund_id: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheckRequest {
    pub acquirer_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheckResponse {
    pub status: AuthorizeStatus,
    pub acquirer_reference: Option<String>,
    pub amount: Option<Money>,
    pub latency_ms: u32,
}

// ─── FX Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateRequest {
    pub source_currency: String,
    pub target_currency: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateResponse {
    pub rate: String,
    pub rate_minor_units: i64,
    pub converted_amount: Money,
    pub fee: Option<Money>,
    pub expires_at: DateTime<Utc>,
}

// ─── 3DS Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check3dsRequest {
    pub card_number: String,
    pub amount: Money,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check3dsResponse {
    pub requires_3ds: bool,
    pub three_ds_data: Option<ThreeDsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticate3dsRequest {
    pub three_ds_data: ThreeDsData,
    pub authentication_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticate3dsResponse {
    pub authenticated: bool,
    pub three_ds_status: String,
    pub eci: Option<String>,
}

// ─── Network Token Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionTokenRequest {
    pub card_number: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cardholder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionTokenResponse {
    pub network_token: String,
    pub token_expiry_month: u32,
    pub token_expiry_year: u32,
    pub cryptogram: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenReference {
    pub network_token: String,
    pub connector_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdateResult {
    pub updated: bool,
    pub new_expiry_month: Option<u32>,
    pub new_expiry_year: Option<u32>,
}

// ─── Settlement Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollSettlementRequest {
    pub since: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSettlementRecord {
    pub transaction_id: String,
    pub amount: Money,
    pub fee: Option<Money>,
    pub settlement_date: DateTime<Utc>,
    pub status: String,
}

// ─── Connector Configuration & Validation Types ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub api_key: Option<String>,
    pub secret_key: Option<String>,
    pub merchant_id: Option<String>,
    pub store_id: Option<String>,
    pub environment: String,
    pub additional_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialValidationResult {
    pub valid: bool,
    pub merchant_name: Option<String>,
    pub permissions: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub merchant_name: Option<String>,
    pub latency_ms: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCardNumber {
    pub label: String,
    pub card_number: String,
    pub scheme: CardScheme,
    pub scenario: String,
}

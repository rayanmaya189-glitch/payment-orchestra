#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};
use shared_types::{CurrencyCode, Money};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLimits {
    pub min_amount: Money,
    pub max_amount: Money,
    pub daily_volume: Money,
    pub monthly_volume: Money,
    pub max_refund_amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub fixed_fee: Money,
    pub percentage_fee_bps: i32,
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,
}

impl FeeStructure {
    pub fn calculate_fee(&self, amount: &Money, is_cross_border: bool, requires_fx: bool) -> Money {
        let percentage_fee = (amount.amount_minor_units * self.percentage_fee_bps as i64) / 10000;
        let cross_border = if is_cross_border {
            (amount.amount_minor_units * self.cross_border_fee_bps as i64) / 10000
        } else {
            0
        };
        let fx_fee = if requires_fx {
            (amount.amount_minor_units * self.currency_conversion_fee_bps as i64) / 10000
        } else {
            0
        };

        Money {
            amount_minor_units: self.fixed_fee.amount_minor_units + percentage_fee + cross_border + fx_fee,
            currency: amount.currency.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub per_second: u32,
    pub per_day: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringThresholds {
    pub success_rate_alert: f64,
    pub success_rate_critical: f64,
    pub latency_p99_alert_ms: u32,
    pub latency_p99_critical_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayProfileStatus {
    Active,
    Disabled,
    Maintenance,
}

impl GatewayProfileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Maintenance => "maintenance",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "disabled" => Self::Disabled,
            "maintenance" => Self::Maintenance,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(uuid::Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
}

impl RotationStrategy {
    pub fn as_str(&self) -> String {
        match self {
            Self::Priority => "priority".to_string(),
            Self::RoundRobin => "round_robin".to_string(),
            Self::WeightedRoundRobin { .. } => "weighted_round_robin".to_string(),
            Self::CostBased => "cost_based".to_string(),
            Self::SuccessRateBased => "success_rate_based".to_string(),
            Self::VolumeCapped => "volume_capped".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "priority" => Self::Priority,
            "round_robin" => Self::RoundRobin,
            "cost_based" => Self::CostBased,
            "success_rate_based" => Self::SuccessRateBased,
            "volume_capped" => Self::VolumeCapped,
            _ => Self::Priority,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizeRequest {
    pub amount: Money,
    pub card_token: String,
    pub currency: CurrencyCode,
    pub merchant_reference: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AuthorizeResponse {
    pub acquirer_reference: String,
    pub status: String,
    pub decline_reason: Option<String>,
    pub fee: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct CaptureRequest {
    pub acquirer_reference: String,
    pub amount: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct CaptureResponse {
    pub status: String,
    pub captured_amount: Money,
}

#[derive(Debug, Clone)]
pub struct VoidRequest {
    pub acquirer_reference: String,
}

#[derive(Debug, Clone)]
pub struct VoidResponse {
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct RefundRequest {
    pub acquirer_reference: String,
    pub amount: Money,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RefundResponse {
    pub status: String,
    pub refund_reference: String,
}

#[derive(Debug, Clone)]
pub struct StatusCheckRequest {
    pub acquirer_reference: String,
}

#[derive(Debug, Clone)]
pub struct StatusCheckResponse {
    pub status: String,
    pub acquirer_reference: String,
}

#[derive(Debug, Clone)]
pub struct PollSettlementRequest {
    pub from_date: chrono::NaiveDate,
    pub to_date: chrono::NaiveDate,
}

#[derive(Debug, Clone)]
pub struct RawSettlementRecord {
    pub acquirer_reference: String,
    pub amount: Money,
    pub settled_at: chrono::DateTime<chrono::Utc>,
    pub fee: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct ConnectorEvent {
    pub event_type: String,
    pub acquirer_reference: String,
    pub amount: Option<Money>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum ConnectorError {
    NetworkError(String),
    AuthenticationError,
    InvalidRequest(String),
    RateLimited,
    Timeout,
    Declined(String),
    Unknown(String),
}

impl std::fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::AuthenticationError => write!(f, "Authentication error"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Declined(msg) => write!(f, "Declined: {msg}"),
            Self::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

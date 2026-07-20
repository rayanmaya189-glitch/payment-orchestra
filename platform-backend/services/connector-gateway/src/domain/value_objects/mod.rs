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

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::CardScheme;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: CurrencyCode::new("AED").unwrap() }
    }

    #[test]
    fn test_fee_calculation_fixed_only() {
        let fee = FeeStructure {
            fixed_fee: aed(100),
            percentage_fee_bps: 0,
            cross_border_fee_bps: 0,
            currency_conversion_fee_bps: 0,
        };
        let result = fee.calculate_fee(&aed(10000), false, false);
        assert_eq!(result.amount_minor_units, 100);
    }

    #[test]
    fn test_fee_calculation_percentage() {
        let fee = FeeStructure {
            fixed_fee: aed(0),
            percentage_fee_bps: 250, // 2.5%
            cross_border_fee_bps: 0,
            currency_conversion_fee_bps: 0,
        };
        let result = fee.calculate_fee(&aed(10000), false, false);
        assert_eq!(result.amount_minor_units, 250); // 10000 * 250 / 10000
    }

    #[test]
    fn test_fee_calculation_cross_border() {
        let fee = FeeStructure {
            fixed_fee: aed(50),
            percentage_fee_bps: 200, // 2%
            cross_border_fee_bps: 100, // 1%
            currency_conversion_fee_bps: 50, // 0.5%
        };
        let result = fee.calculate_fee(&aed(10000), true, true);
        // 50 + 200 + 100 + 50 = 400
        assert_eq!(result.amount_minor_units, 400);
    }

    #[test]
    fn test_fee_calculation_no_cross_border_no_fx() {
        let fee = FeeStructure {
            fixed_fee: aed(50),
            percentage_fee_bps: 200,
            cross_border_fee_bps: 100,
            currency_conversion_fee_bps: 50,
        };
        let result = fee.calculate_fee(&aed(10000), false, false);
        // 50 + 200 + 0 + 0 = 250
        assert_eq!(result.amount_minor_units, 250);
    }

    #[test]
    fn test_gateway_profile_validate_transaction() {
        use crate::domain::aggregates::GatewayProfile;
        use chrono::Utc;

        let profile = GatewayProfile {
            profile_id: uuid::Uuid::now_v7(),
            operator_id: uuid::Uuid::now_v7(),
            connector_id: "test".to_string(),
            merchant_acquirer_link_id: uuid::Uuid::now_v7(),
            status: GatewayProfileStatus::Active,
            min_transaction_amount_minor: 100,
            max_transaction_amount_minor: 50_000_000,
            daily_volume_limit_minor: 5_000_000_000,
            monthly_volume_limit_minor: 50_000_000_000,
            max_refund_amount_minor: 50_000_000,
            fixed_fee_minor: 100,
            percentage_fee_bps: 250,
            cross_border_fee_bps: 0,
            currency_conversion_fee_bps: 0,
            routing_priority: 1,
            base_url: "https://api.test.com".to_string(),
            enabled_card_schemes: vec![CardScheme::Visa],
            enabled_currencies: vec![CurrencyCode::new("AED").unwrap()],
            enabled_countries: vec!["AE".to_string()],
            rate_limit_per_second: 100,
            rate_limit_per_day: 1_000_000,
            success_rate_threshold: 0.95,
            latency_threshold_ms: 5000,
            auto_disable_on_low_success: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Valid transaction
        assert!(profile.validate_transaction(&aed(5000), &CardScheme::Visa, &CurrencyCode::new("AED").unwrap()).is_ok());

        // Below minimum
        assert!(profile.validate_transaction(&aed(50), &CardScheme::Visa, &CurrencyCode::new("AED").unwrap()).is_err());

        // Unsupported card scheme
        assert!(profile.validate_transaction(&aed(5000), &CardScheme::Amex, &CurrencyCode::new("AED").unwrap()).is_err());

        // Unsupported currency
        assert!(profile.validate_transaction(&aed(5000), &CardScheme::Visa, &CurrencyCode::new("USD").unwrap()).is_err());
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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "maintenance" => Ok(Self::Maintenance),
            _ => Err("unknown gateway profile status"),
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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "priority" => Ok(Self::Priority),
            "round_robin" => Ok(Self::RoundRobin),
            "cost_based" => Ok(Self::CostBased),
            "success_rate_based" => Ok(Self::SuccessRateBased),
            "volume_capped" => Ok(Self::VolumeCapped),
            _ => Err("unknown rotation strategy"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizeRequest {
    pub amount: Money,
    pub card_token: String,
    pub currency: CurrencyCode,
    pub merchant_reference: String,
    pub idempotency_key: String,
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

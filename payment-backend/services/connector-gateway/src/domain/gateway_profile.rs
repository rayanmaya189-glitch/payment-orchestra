use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ConnectorError;
use super::types::{CardScheme, Money};

// ─── Gateway Profile ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProfile {
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub status: ProfileStatus,
    pub limits: TransactionLimits,
    pub fees: FeeStructure,
    pub routing_priority: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<String>,
    pub enabled_countries: Vec<String>,
    pub rate_limits: RateLimitConfig,
    pub monitoring: MonitoringThresholds,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileStatus {
    Active,
    Disabled,
    Maintenance,
}

impl ProfileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileStatus::Active => "active",
            ProfileStatus::Disabled => "disabled",
            ProfileStatus::Maintenance => "maintenance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLimits {
    pub min_amount_minor: i64,
    pub max_amount_minor: i64,
    pub daily_volume_limit_minor: i64,
    pub monthly_volume_limit_minor: i64,
    pub max_refund_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub fixed_fee_minor: i64,
    pub percentage_fee_bps: i32,
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,
    pub max_fee_cap: Option<i64>,
    pub min_fee_floor: Option<i64>,
    pub tiered_pricing: Option<Vec<FeeTier>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTier {
    pub min_volume_minor: i64,
    pub max_volume_minor: Option<i64>,
    pub percentage_fee_bps: i32,
}

impl FeeStructure {
    pub fn calculate_fee(&self, amount: &Money, is_cross_border: bool, requires_fx: bool, daily_volume: i64) -> Money {
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

        let mut total = self.fixed_fee_minor + percentage_fee + cross_border + fx_fee;

        // Apply tiered pricing if configured
        if let Some(tiers) = &self.tiered_pricing {
            if let Some(tier) = tiers.iter().find(|t| {
                daily_volume >= t.min_volume_minor
                    && t.max_volume_minor.is_none_or(|max| daily_volume < max)
            }) {
                total = self.fixed_fee_minor
                    + (amount.amount_minor_units * tier.percentage_fee_bps as i64) / 10000
                    + cross_border
                    + fx_fee;
            }
        }

        // Apply fee cap
        if let Some(cap) = self.max_fee_cap {
            total = total.min(cap);
        }

        // Apply fee floor
        if let Some(floor) = self.min_fee_floor {
            total = total.max(floor);
        }

        Money {
            amount_minor_units: total,
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
    pub auto_disable_on_low_success: bool,
}

// ─── Validation Function ─────────────────────────────────────────────────────

pub fn validate_transaction_against_profile(
    amount: &Money,
    profile: &GatewayProfile,
    card_scheme: &CardScheme,
    currency: &str,
) -> Result<(), ConnectorError> {
    if amount.amount_minor_units < profile.limits.min_amount_minor {
        return Err(ConnectorError::InvalidRequest("Below minimum amount".into()));
    }
    if amount.amount_minor_units > profile.limits.max_amount_minor {
        return Err(ConnectorError::InvalidRequest("Exceeds maximum amount".into()));
    }
    if !profile.enabled_card_schemes.contains(card_scheme) {
        return Err(ConnectorError::InvalidRequest("Unsupported card scheme".into()));
    }
    if !profile.enabled_currencies.iter().any(|c| c == currency) {
        return Err(ConnectorError::InvalidRequest("Unsupported currency".into()));
    }
    Ok(())
}

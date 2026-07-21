use serde::{Deserialize, Serialize};

/// The decision resulting from a risk assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskDecision {
    Allow,
    Review,
    Decline,
}

impl RiskDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Review => "review",
            Self::Decline => "decline",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "review" => Self::Review,
            "decline" => Self::Decline,
            _ => Self::Allow,
        }
    }
}

/// A single factor contributing to the overall risk score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// The rule name that produced this factor (e.g. "high_amount").
    pub rule_name: String,
    /// The raw score contributed by this factor (0.0–1.0).
    pub score: f64,
    /// The weight applied to this factor in the final calculation.
    pub weight: f64,
    /// Human-readable explanation of why this factor was triggered.
    pub description: String,
}

/// Contextual information about the payment being assessed.
#[derive(Debug, Clone)]
pub struct PaymentContext {
    pub payment_intent_id: uuid::Uuid,
    pub operator_id: uuid::Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    /// Country code derived from IP geolocation (ISO 3166-1 alpha-2).
    pub country_code: Option<String>,
    /// Merchant's home country code for geo-mismatch checks.
    pub merchant_country: Option<String>,
    /// Whether the entity (cardholder IP / card fingerprint) is whitelisted.
    pub is_whitelisted: bool,
    /// Whether the entity is blacklisted.
    pub is_blacklisted: bool,
    /// Recent transaction count from the same IP in the velocity window.
    pub recent_tx_count_from_ip: u32,
    /// Recent transaction count from the same card fingerprint in the velocity window.
    pub recent_tx_count_from_card: u32,
}

/// Configuration thresholds for risk rules.
#[derive(Debug, Clone)]
pub struct RiskThresholds {
    /// Amount above which the high-amount rule fires (in minor units).
    pub high_amount_threshold: i64,
    /// Max transactions from same IP within the velocity window.
    pub velocity_ip_max: u32,
    /// Max transactions from same card within the velocity window.
    pub velocity_card_max: u32,
    /// Countries allowed for the merchant (ISO codes). Empty = no restriction.
    pub allowed_countries: Vec<String>,
    /// Score thresholds for decision mapping.
    pub review_threshold: f64,
    pub decline_threshold: f64,
}

impl Default for RiskThresholds {
    fn default() -> Self {
        Self {
            high_amount_threshold: 500_00, // 500.00 in minor units (2 decimals)
            velocity_ip_max: 10,
            velocity_card_max: 5,
            allowed_countries: Vec::new(),
            review_threshold: 0.4,
            decline_threshold: 0.7,
        }
    }
}

/// Breakdown of individual factor scores for transparency.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskFactorBreakdown {
    pub amount_factor: f64,
    pub velocity_factor: f64,
    pub geo_factor: f64,
    pub blacklist_factor: f64,
    pub whitelist_override: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_decision_as_str() {
        assert_eq!(RiskDecision::Allow.as_str(), "allow");
        assert_eq!(RiskDecision::Review.as_str(), "review");
        assert_eq!(RiskDecision::Decline.as_str(), "decline");
    }

    #[test]
    fn test_risk_decision_from_str() {
        assert_eq!(RiskDecision::from_str("allow"), RiskDecision::Allow);
        assert_eq!(RiskDecision::from_str("review"), RiskDecision::Review);
        assert_eq!(RiskDecision::from_str("decline"), RiskDecision::Decline);
        assert_eq!(RiskDecision::from_str("unknown"), RiskDecision::Allow);
    }

    #[test]
    fn test_default_thresholds() {
        let t = RiskThresholds::default();
        assert_eq!(t.high_amount_threshold, 500_00);
        assert_eq!(t.velocity_ip_max, 10);
        assert_eq!(t.velocity_card_max, 5);
        assert!(t.allowed_countries.is_empty());
        assert!((t.review_threshold - 0.4).abs() < f64::EPSILON);
        assert!((t.decline_threshold - 0.7).abs() < f64::EPSILON);
    }
}

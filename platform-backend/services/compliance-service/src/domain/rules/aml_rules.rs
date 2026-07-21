//! AML (Anti-Money Laundering) monitoring rules.
//!
//! Implements transaction monitoring rules per SRS Part 8 §11.1.
//! Each rule checks a transaction against specific patterns and generates
//! alerts when suspicious activity is detected.

use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;

/// An AML alert generated when a rule is triggered.
#[derive(Debug, Clone)]
pub struct AmlAlert {
    pub alert_id: Uuid,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AmlSeverity,
    pub principal_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub description: String,
    pub factors: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// AML alert severity levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmlSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AmlSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Transaction context for AML rule evaluation.
#[derive(Debug, Clone)]
pub struct TransactionContext {
    pub transaction_id: Uuid,
    pub principal_id: Uuid,
    pub amount: Money,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub country: Option<String>,
    pub recipient: Option<String>,
}

/// Trait for AML rules.
pub trait AmlRule: Send + Sync {
    /// Evaluate a transaction against this rule.
    /// Returns Some(alert) if the rule is triggered, None otherwise.
    fn evaluate(&self, ctx: &TransactionContext, history: &[TransactionContext]) -> Option<AmlAlert>;
}

/// AML-001: Structuring — multiple transactions just below reporting threshold.
///
/// Checks if the principal has made multiple transactions in a short window
/// that individually stay below the threshold but collectively exceed it.
pub struct StructuringRule {
    pub threshold_amount_minor: i64,
    pub window_hours: i64,
    pub min_transactions: u32,
}

impl StructuringRule {
    pub fn new() -> Self {
        Self {
            threshold_amount_minor: 10_000_000, // 100,000 AED
            window_hours: 24,
            min_transactions: 3,
        }
    }
}

impl AmlRule for StructuringRule {
    fn evaluate(&self, ctx: &TransactionContext, history: &[TransactionContext]) -> Option<AmlAlert> {
        let window_start = ctx.timestamp - chrono::Duration::hours(self.window_hours);

        let recent: Vec<&TransactionContext> = history.iter()
            .filter(|t| t.principal_id == ctx.principal_id)
            .filter(|t| t.timestamp >= window_start)
            .filter(|t| t.amount.amount_minor_units < self.threshold_amount_minor)
            .collect();

        if recent.len() as u32 >= self.min_transactions {
            let total: i64 = recent.iter().map(|t| t.amount.amount_minor_units).sum();
            if total >= self.threshold_amount_minor {
                return Some(AmlAlert {
                    alert_id: Uuid::now_v7(),
                    rule_id: "AML-001".to_string(),
                    rule_name: "Structuring".to_string(),
                    severity: AmlSeverity::High,
                    principal_id: ctx.principal_id,
                    transaction_id: Some(ctx.transaction_id),
                    description: format!(
                        "Multiple transactions below threshold detected: {} transactions totaling {} minor units in {} hours",
                        recent.len(), total, self.window_hours
                    ),
                    factors: vec![
                        format!("transaction_count={}", recent.len()),
                        format!("total_amount={}", total),
                        format!("window_hours={}", self.window_hours),
                    ],
                    created_at: Utc::now(),
                });
            }
        }

        None
    }
}

/// AML-002: Velocity — unusually high transaction frequency.
pub struct VelocityRule {
    pub max_transactions_per_hour: u32,
}

impl VelocityRule {
    pub fn new() -> Self {
        Self {
            max_transactions_per_hour: 50,
        }
    }
}

impl AmlRule for VelocityRule {
    fn evaluate(&self, ctx: &TransactionContext, history: &[TransactionContext]) -> Option<AmlAlert> {
        let window_start = ctx.timestamp - chrono::Duration::hours(1);
        let count = history.iter()
            .filter(|t| t.principal_id == ctx.principal_id)
            .filter(|t| t.timestamp >= window_start)
            .count() as u32;

        if count >= self.max_transactions_per_hour {
            return Some(AmlAlert {
                alert_id: Uuid::now_v7(),
                rule_id: "AML-002".to_string(),
                rule_name: "Velocity".to_string(),
                severity: AmlSeverity::Medium,
                principal_id: ctx.principal_id,
                transaction_id: Some(ctx.transaction_id),
                description: format!(
                    "High transaction velocity: {} transactions in the last hour (threshold: {})",
                    count, self.max_transactions_per_hour
                ),
                factors: vec![
                    format!("transactions_per_hour={}", count),
                    format!("threshold={}", self.max_transactions_per_hour),
                ],
                created_at: Utc::now(),
            });
        }

        None
    }
}

/// AML-003: Geographic anomaly — transactions from unexpected countries.
pub struct GeoAnomalyRule {
    pub allowed_countries: Vec<String>,
}

impl GeoAnomalyRule {
    pub fn new(allowed_countries: Vec<String>) -> Self {
        Self { allowed_countries }
    }
}

impl AmlRule for GeoAnomalyRule {
    fn evaluate(&self, ctx: &TransactionContext, _history: &[TransactionContext]) -> Option<AmlAlert> {
        if let Some(ref country) = ctx.country {
            if !self.allowed_countries.contains(country) {
                return Some(AmlAlert {
                    alert_id: Uuid::now_v7(),
                    rule_id: "AML-003".to_string(),
                    rule_name: "Geographic Anomaly".to_string(),
                    severity: AmlSeverity::Medium,
                    principal_id: ctx.principal_id,
                    transaction_id: Some(ctx.transaction_id),
                    description: format!(
                        "Transaction from unexpected country: {} (allowed: {:?})",
                        country, self.allowed_countries
                    ),
                    factors: vec![
                        format!("country={}", country),
                        format!("allowed={:?}", self.allowed_countries),
                    ],
                    created_at: Utc::now(),
                });
            }
        }

        None
    }
}

/// AML-004: Rapid succession — multiple transactions in very short time.
pub struct RapidSuccessionRule {
    pub max_seconds_between: u64,
    pub min_transactions: u32,
}

impl RapidSuccessionRule {
    pub fn new() -> Self {
        Self {
            max_seconds_between: 60,
            min_transactions: 5,
        }
    }
}

impl AmlRule for RapidSuccessionRule {
    fn evaluate(&self, ctx: &TransactionContext, history: &[TransactionContext]) -> Option<AmlAlert> {
        let window_start = ctx.timestamp - chrono::Duration::seconds(self.max_seconds_between as i64);
        let count = history.iter()
            .filter(|t| t.principal_id == ctx.principal_id)
            .filter(|t| t.timestamp >= window_start)
            .count() as u32;

        if count >= self.min_transactions {
            return Some(AmlAlert {
                alert_id: Uuid::now_v7(),
                rule_id: "AML-004".to_string(),
                rule_name: "Rapid Succession".to_string(),
                severity: AmlSeverity::High,
                principal_id: ctx.principal_id,
                transaction_id: Some(ctx.transaction_id),
                description: format!(
                    "Rapid succession: {} transactions within {} seconds",
                    count, self.max_seconds_between
                ),
                factors: vec![
                    format!("transaction_count={}", count),
                    format!("window_seconds={}", self.max_seconds_between),
                ],
                created_at: Utc::now(),
            });
        }

        None
    }
}

/// AML-005: High-risk country — transactions involving sanctioned countries.
pub struct HighRiskCountryRule {
    pub sanctioned_countries: Vec<String>,
}

impl HighRiskCountryRule {
    pub fn new() -> Self {
        Self {
            sanctioned_countries: vec![
                "KP".to_string(), // North Korea
                "IR".to_string(), // Iran
                "SY".to_string(), // Syria
                "CU".to_string(), // Cuba
            ],
        }
    }
}

impl AmlRule for HighRiskCountryRule {
    fn evaluate(&self, ctx: &TransactionContext, _history: &[TransactionContext]) -> Option<AmlAlert> {
        if let Some(ref country) = ctx.country {
            if self.sanctioned_countries.contains(country) {
                return Some(AmlAlert {
                    alert_id: Uuid::now_v7(),
                    rule_id: "AML-005".to_string(),
                    rule_name: "Sanctioned Country".to_string(),
                    severity: AmlSeverity::Critical,
                    principal_id: ctx.principal_id,
                    transaction_id: Some(ctx.transaction_id),
                    description: format!(
                        "Transaction involves sanctioned country: {}",
                        country
                    ),
                    factors: vec![
                        format!("country={}", country),
                        format!("sanctioned={:?}", self.sanctioned_countries),
                    ],
                    created_at: Utc::now(),
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx(amount: i64, country: &str) -> TransactionContext {
        TransactionContext {
            transaction_id: Uuid::now_v7(),
            principal_id: Uuid::now_v7(),
            amount: Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() },
            timestamp: Utc::now(),
            ip_address: Some("1.2.3.4".to_string()),
            country: Some(country.to_string()),
            recipient: None,
        }
    }

    #[test]
    fn test_structuring_rule_no_alert() {
        let rule = StructuringRule::new();
        let ctx = test_ctx(5_000_000, "AE"); // 50,000 AED
        assert!(rule.evaluate(&ctx, &[]).is_none());
    }

    #[test]
    fn test_velocity_rule_no_alert() {
        let rule = VelocityRule::new();
        let ctx = test_ctx(1000, "AE");
        assert!(rule.evaluate(&ctx, &[]).is_none());
    }

    #[test]
    fn test_geo_anomaly_rule_detects() {
        let rule = GeoAnomalyRule::new(vec!["AE".to_string(), "SA".to_string()]);
        let ctx = test_ctx(1000, "KP"); // Sanctioned
        let alert = rule.evaluate(&ctx, &[]);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().rule_id, "AML-003");
    }

    #[test]
    fn test_high_risk_country_rule_detects() {
        let rule = HighRiskCountryRule::new();
        let ctx = test_ctx(1000, "KP"); // North Korea
        let alert = rule.evaluate(&ctx, &[]);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, AmlSeverity::Critical);
    }

    #[test]
    fn test_high_risk_country_rule_allows() {
        let rule = HighRiskCountryRule::new();
        let ctx = test_ctx(1000, "AE"); // UAE
        assert!(rule.evaluate(&ctx, &[]).is_none());
    }
}

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::RiskAssessment;
use crate::domain::value_objects::{PaymentContext, RiskFactor, RiskThresholds};
use platform_error::PlatformError;

// ---------------------------------------------------------------------------
// Repository trait (placed here to keep domain layer self-contained)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RiskAssessmentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, PlatformError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), PlatformError>;
}

// ---------------------------------------------------------------------------
// Velocity look-up port (injected by infrastructure layer)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait VelocityLookup: Send + Sync {
    /// Return (tx_count_from_ip, tx_count_from_card) within the configured window.
    async fn recent_counts(
        &self,
        ip_address: &str,
        card_fingerprint: &str,
        window_seconds: u64,
    ) -> Result<(u32, u32), PlatformError>;
}

// ---------------------------------------------------------------------------
// Geo-lookup port
// ---------------------------------------------------------------------------

#[async_trait]
pub trait GeoLookup: Send + Sync {
    /// Resolve an IP address to an ISO 3166-1 alpha-2 country code.
    async fn resolve_country(&self, ip: &str) -> Result<Option<String>, PlatformError>;
}

// ---------------------------------------------------------------------------
// Blacklist / Whitelist ports
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EntityListStore: Send + Sync {
    async fn is_blacklisted(&self, entity_key: &str) -> Result<bool, PlatformError>;
    async fn is_whitelisted(&self, entity_key: &str) -> Result<bool, PlatformError>;
}

// ---------------------------------------------------------------------------
// RiskRule trait
// ---------------------------------------------------------------------------

/// A single risk rule that inspects a `PaymentContext` and optionally returns
/// a `RiskFactor` contributing to the overall score.
pub trait RiskRule: Send + Sync {
    /// Human-readable name of the rule (used as `RiskFactor.rule_name`).
    fn name(&self) -> &str;

    /// Evaluate the rule against the given context. Returns `Some(factor)` when
    /// the rule fires, `None` otherwise.
    fn evaluate(&self, ctx: &PaymentContext, thresholds: &RiskThresholds) -> Option<RiskFactor>;
}

// ===========================================================================
// Concrete rules
// ===========================================================================

/// Flags transactions whose amount exceeds a configurable threshold.
pub struct HighAmountRule;

impl RiskRule for HighAmountRule {
    fn name(&self) -> &str {
        "high_amount"
    }

    fn evaluate(&self, ctx: &PaymentContext, thresholds: &RiskThresholds) -> Option<RiskFactor> {
        if ctx.amount_minor_units > thresholds.high_amount_threshold {
            let ratio = ctx.amount_minor_units as f64 / thresholds.high_amount_threshold as f64;
            // Score ramps from 0.0 (at threshold) toward 1.0 (at 5× threshold), capped at 1.0.
            let score = ((ratio - 1.0) / 4.0).clamp(0.0, 1.0);
            Some(RiskFactor {
                rule_name: self.name().to_string(),
                score,
                weight: 0.30,
                description: format!(
                    "Amount {} exceeds threshold {}",
                    ctx.amount_minor_units, thresholds.high_amount_threshold
                ),
            })
        } else {
            None
        }
    }
}

/// Flags excessive transaction velocity from the same IP address or card.
pub struct VelocityRule;

impl RiskRule for VelocityRule {
    fn name(&self) -> &str {
        "velocity"
    }

    fn evaluate(&self, ctx: &PaymentContext, thresholds: &RiskThresholds) -> Option<RiskFactor> {
        let ip_ratio = if thresholds.velocity_ip_max > 0 {
            ctx.recent_tx_count_from_ip as f64 / thresholds.velocity_ip_max as f64
        } else {
            0.0
        };
        let card_ratio = if thresholds.velocity_card_max > 0 {
            ctx.recent_tx_count_from_card as f64 / thresholds.velocity_card_max as f64
        } else {
            0.0
        };

        let max_ratio = ip_ratio.max(card_ratio);
        if max_ratio > 1.0 {
            let score = ((max_ratio - 1.0) / 4.0).clamp(0.0, 1.0);
            Some(RiskFactor {
                rule_name: self.name().to_string(),
                score,
                weight: 0.25,
                description: format!(
                    "Velocity exceeded: IP count={}, card count={}, limits={}/{}",
                    ctx.recent_tx_count_from_ip,
                    ctx.recent_tx_count_from_card,
                    thresholds.velocity_ip_max,
                    thresholds.velocity_card_max,
                ),
            })
        } else {
            None
        }
    }
}

/// Flags transactions originating from a country not in the merchant's allowed list.
pub struct GeoMismatchRule;

impl RiskRule for GeoMismatchRule {
    fn name(&self) -> &str {
        "geo_mismatch"
    }

    fn evaluate(&self, ctx: &PaymentContext, thresholds: &RiskThresholds) -> Option<RiskFactor> {
        // If no allowed countries configured, geo check is disabled.
        if thresholds.allowed_countries.is_empty() {
            return None;
        }

        let country = match &ctx.country_code {
            Some(c) => c,
            None => {
                // Unknown geo is suspicious when allowed list is defined.
                return Some(RiskFactor {
                    rule_name: self.name().to_string(),
                    score: 0.5,
                    weight: 0.20,
                    description: "Transaction country could not be resolved".to_string(),
                });
            }
        };

        if !thresholds.allowed_countries.contains(country) {
            Some(RiskFactor {
                rule_name: self.name().to_string(),
                score: 0.8,
                weight: 0.20,
                description: format!(
                    "Country {} is not in the merchant's allowed list",
                    country
                ),
            })
        } else {
            None
        }
    }
}

/// Checks whether the transaction entity is on the blacklist.
pub struct BlacklistRule;

impl RiskRule for BlacklistRule {
    fn name(&self) -> &str {
        "blacklist"
    }

    fn evaluate(&self, ctx: &PaymentContext, _thresholds: &RiskThresholds) -> Option<RiskFactor> {
        if ctx.is_blacklisted {
            Some(RiskFactor {
                rule_name: self.name().to_string(),
                score: 1.0,
                weight: 1.0,
                description: "Entity is blacklisted".to_string(),
            })
        } else {
            None
        }
    }
}

/// Checks whether the transaction entity is on the whitelist (overrides score).
pub struct WhitelistRule;

impl RiskRule for WhitelistRule {
    fn name(&self) -> &str {
        "whitelist"
    }

    fn evaluate(&self, ctx: &PaymentContext, _thresholds: &RiskThresholds) -> Option<RiskFactor> {
        if ctx.is_whitelisted {
            Some(RiskFactor {
                rule_name: self.name().to_string(),
                score: 0.0,
                weight: 0.0,
                description: "Entity is whitelisted — risk score overridden".to_string(),
            })
        } else {
            None
        }
    }
}

/// Returns the default set of risk rules in evaluation order.
pub fn default_rules() -> Vec<Box<dyn RiskRule>> {
    vec![
        Box::new(BlacklistRule),
        Box::new(WhitelistRule),
        Box::new(HighAmountRule),
        Box::new(VelocityRule),
        Box::new(GeoMismatchRule),
    ]
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx() -> PaymentContext {
        PaymentContext {
            payment_intent_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 100_00, // 100.00
            currency: "AED".to_string(),
            ip_address: Some("203.0.113.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            country_code: Some("AE".to_string()),
            merchant_country: Some("AE".to_string()),
            is_whitelisted: false,
            is_blacklisted: false,
            recent_tx_count_from_ip: 1,
            recent_tx_count_from_card: 1,
        }
    }

    // ---- HighAmountRule ---------------------------------------------------

    #[test]
    fn high_amount_below_threshold() {
        let ctx = base_ctx();
        let t = RiskThresholds::default();
        assert!(HighAmountRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn high_amount_above_threshold() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 600_00; // above 500.00
        let t = RiskThresholds::default();
        let factor = HighAmountRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.rule_name, "high_amount");
        assert!(factor.score > 0.0);
        assert!(factor.score <= 1.0);
    }

    #[test]
    fn high_amount_at_exact_threshold() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 500_00; // exactly at threshold
        let t = RiskThresholds::default();
        // At exactly the threshold the rule should NOT fire (> not >=).
        assert!(HighAmountRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn high_amount_score_caps_at_one() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 10_000_00; // 10,000.00 — well above threshold
        let t = RiskThresholds::default();
        let factor = HighAmountRule.evaluate(&ctx, &t).unwrap();
        assert!(factor.score <= 1.0);
    }

    // ---- VelocityRule -----------------------------------------------------

    #[test]
    fn velocity_within_limits() {
        let ctx = base_ctx();
        let t = RiskThresholds::default();
        assert!(VelocityRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn velocity_ip_exceeded() {
        let mut ctx = base_ctx();
        ctx.recent_tx_count_from_ip = 15; // above limit of 10
        let t = RiskThresholds::default();
        let factor = VelocityRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.rule_name, "velocity");
        assert!(factor.score > 0.0);
    }

    #[test]
    fn velocity_card_exceeded() {
        let mut ctx = base_ctx();
        ctx.recent_tx_count_from_card = 8; // above limit of 5
        let t = RiskThresholds::default();
        let factor = VelocityRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.rule_name, "velocity");
    }

    #[test]
    fn velocity_takes_worse_of_ip_and_card() {
        let mut ctx = base_ctx();
        ctx.recent_tx_count_from_ip = 15; // ratio 1.5
        ctx.recent_tx_count_from_card = 10; // ratio 2.0
        let t = RiskThresholds::default();
        let factor = VelocityRule.evaluate(&ctx, &t).unwrap();
        // Card ratio (2.0) is worse than IP ratio (1.5)
        assert!(factor.score > 0.0);
    }

    // ---- GeoMismatchRule --------------------------------------------------

    #[test]
    fn geo_match_no_flag() {
        let mut ctx = base_ctx();
        ctx.country_code = Some("AE".to_string());
        let mut t = RiskThresholds::default();
        t.allowed_countries = vec!["AE".to_string(), "SA".to_string()];
        assert!(GeoMismatchRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn geo_mismatch_flags() {
        let mut ctx = base_ctx();
        ctx.country_code = Some("XX".to_string());
        let mut t = RiskThresholds::default();
        t.allowed_countries = vec!["AE".to_string(), "SA".to_string()];
        let factor = GeoMismatchRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.rule_name, "geo_mismatch");
        assert!((factor.score - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn geo_unknown_country_suspicious() {
        let mut ctx = base_ctx();
        ctx.country_code = None;
        let mut t = RiskThresholds::default();
        t.allowed_countries = vec!["AE".to_string()];
        let factor = GeoMismatchRule.evaluate(&ctx, &t).unwrap();
        assert!((factor.score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn geo_disabled_when_no_allowed_countries() {
        let ctx = base_ctx();
        let t = RiskThresholds::default(); // allowed_countries is empty
        assert!(GeoMismatchRule.evaluate(&ctx, &t).is_none());
    }

    // ---- BlacklistRule ----------------------------------------------------

    #[test]
    fn blacklist_not_flagged() {
        let ctx = base_ctx();
        let t = RiskThresholds::default();
        assert!(BlacklistRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn blacklist_flagged() {
        let mut ctx = base_ctx();
        ctx.is_blacklisted = true;
        let t = RiskThresholds::default();
        let factor = BlacklistRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.score, 1.0);
        assert_eq!(factor.weight, 1.0);
    }

    // ---- WhitelistRule ----------------------------------------------------

    #[test]
    fn whitelist_not_flagged() {
        let ctx = base_ctx();
        let t = RiskThresholds::default();
        assert!(WhitelistRule.evaluate(&ctx, &t).is_none());
    }

    #[test]
    fn whitelist_flagged() {
        let mut ctx = base_ctx();
        ctx.is_whitelisted = true;
        let t = RiskThresholds::default();
        let factor = WhitelistRule.evaluate(&ctx, &t).unwrap();
        assert_eq!(factor.score, 0.0);
        assert_eq!(factor.weight, 0.0);
    }

    // ---- default_rules ----------------------------------------------------

    #[test]
    fn default_rules_count() {
        let rules = default_rules();
        assert_eq!(rules.len(), 5);
    }
}

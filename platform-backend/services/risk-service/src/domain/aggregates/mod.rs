use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::rules::RiskRule;
use crate::domain::value_objects::{
    PaymentContext, RiskDecision, RiskFactor, RiskFactorBreakdown, RiskThresholds,
};

/// The core risk-assessment aggregate.
///
/// Created for each payment intent, evaluated against a set of rules, and
/// persisted so that downstream services can query the decision.
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    /// Aggregate risk score in the range 0.0–1.0.
    pub score: f64,
    /// Decision derived from the score (allow / review / decline).
    pub decision: RiskDecision,
    /// Individual factors that contributed to the score.
    pub factors: Vec<RiskFactor>,
    /// Per-category breakdown for quick inspection.
    pub breakdown: RiskFactorBreakdown,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl RiskAssessment {
    pub fn new(payment_intent_id: Uuid, operator_id: Uuid) -> Self {
        Self {
            assessment_id: Uuid::now_v7(),
            payment_intent_id,
            operator_id,
            score: 0.0,
            decision: RiskDecision::Allow,
            factors: Vec::new(),
            breakdown: RiskFactorBreakdown::default(),
            ip_address: None,
            user_agent: None,
            created_at: Utc::now(),
        }
    }

    /// Apply all rules to the payment context, compute the weighted score, and
    /// derive the decision.
    pub fn evaluate(&mut self, ctx: &PaymentContext, rules: &[Box<dyn RiskRule>], thresholds: &RiskThresholds) {
        self.factors.clear();
        self.breakdown = RiskFactorBreakdown::default();

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for rule in rules {
            if let Some(factor) = rule.evaluate(ctx, thresholds) {
                match factor.rule_name.as_str() {
                    "high_amount" => self.breakdown.amount_factor = factor.score,
                    "velocity" => self.breakdown.velocity_factor = factor.score,
                    "geo_mismatch" => self.breakdown.geo_factor = factor.score,
                    "blacklist" => self.breakdown.blacklist_factor = factor.score,
                    "whitelist" => self.breakdown.whitelist_override = true,
                    _ => {}
                }
                weighted_sum += factor.score * factor.weight;
                total_weight += factor.weight;
                self.factors.push(factor);
            }
        }

        // Normalise to 0.0–1.0 using weighted average, then clamp.
        self.score = if total_weight > 0.0 {
            (weighted_sum / total_weight).clamp(0.0, 1.0)
        } else {
            0.0
        };

        self.decision = self.decide();
    }

    /// Map the current score to a decision, respecting blacklist/whitelist
    /// overrides. Whitelist takes precedence over blacklist.
    pub fn decide(&self) -> RiskDecision {
        if self.is_whitelisted() {
            return RiskDecision::Allow;
        }
        if self.is_blacklisted() {
            return RiskDecision::Decline;
        }
        if self.score <= 0.4 {
            RiskDecision::Allow
        } else if self.score <= 0.7 {
            RiskDecision::Review
        } else {
            RiskDecision::Decline
        }
    }

    /// Whether the blacklist factor was triggered.
    fn is_blacklisted(&self) -> bool {
        self.factors.iter().any(|f| f.rule_name == "blacklist")
    }

    /// Whether the whitelist factor was triggered.
    fn is_whitelisted(&self) -> bool {
        self.breakdown.whitelist_override
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rules::{
        BlacklistRule, GeoMismatchRule, HighAmountRule, VelocityRule, WhitelistRule,
    };

    fn default_thresholds() -> RiskThresholds {
        RiskThresholds::default()
    }

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

    fn all_rules() -> Vec<Box<dyn RiskRule>> {
        vec![
            Box::new(BlacklistRule),
            Box::new(WhitelistRule),
            Box::new(HighAmountRule),
            Box::new(VelocityRule),
            Box::new(GeoMismatchRule),
        ]
    }

    // ---- New assessment ---------------------------------------------------

    #[test]
    fn new_assessment_defaults() {
        let a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        assert_eq!(a.score, 0.0);
        assert_eq!(a.decision, RiskDecision::Allow);
        assert!(a.factors.is_empty());
    }

    // ---- Evaluate: no rules fire ------------------------------------------

    #[test]
    fn evaluate_no_rules_fire() {
        let ctx = base_ctx();
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());
        assert_eq!(a.score, 0.0);
        assert_eq!(a.decision, RiskDecision::Allow);
        assert!(a.factors.is_empty());
    }

    // ---- Evaluate: high amount --------------------------------------------

    #[test]
    fn evaluate_high_amount_only() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 800_00; // 800.00 — above 500.00 threshold
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert!(a.score > 0.0);
        assert!(a.factors.iter().any(|f| f.rule_name == "high_amount"));
        assert!((a.breakdown.amount_factor - a.factors.iter().find(|f| f.rule_name == "high_amount").unwrap().score).abs() < f64::EPSILON);
    }

    // ---- Evaluate: velocity ------------------------------------------------

    #[test]
    fn evaluate_velocity_only() {
        let mut ctx = base_ctx();
        ctx.recent_tx_count_from_ip = 20; // above 10
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert!(a.score > 0.0);
        assert!(a.factors.iter().any(|f| f.rule_name == "velocity"));
    }

    // ---- Evaluate: geo mismatch -------------------------------------------

    #[test]
    fn evaluate_geo_mismatch_only() {
        let mut ctx = base_ctx();
        ctx.country_code = Some("XX".to_string());
        let mut t = default_thresholds();
        t.allowed_countries = vec!["AE".to_string(), "SA".to_string()];
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &t);

        assert!(a.score > 0.0);
        assert!(a.factors.iter().any(|f| f.rule_name == "geo_mismatch"));
    }

    // ---- Evaluate: multiple rules ------------------------------------------

    #[test]
    fn evaluate_multiple_rules_accumulate() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 800_00;
        ctx.recent_tx_count_from_ip = 20;
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert!(a.score > 0.0);
        assert!(a.factors.len() >= 2);
    }

    // ---- Decide: score thresholds -----------------------------------------

    #[test]
    fn decide_allow_below_review() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.score = 0.3;
        assert_eq!(a.decide(), RiskDecision::Allow);
    }

    #[test]
    fn decide_review_in_middle() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.score = 0.55;
        assert_eq!(a.decide(), RiskDecision::Review);
    }

    #[test]
    fn decide_decline_above_threshold() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.score = 0.85;
        assert_eq!(a.decide(), RiskDecision::Decline);
    }

    #[test]
    fn decide_exact_boundary_review() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.score = 0.4;
        assert_eq!(a.decide(), RiskDecision::Allow);
    }

    #[test]
    fn decide_exact_boundary_decline() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.score = 0.7;
        assert_eq!(a.decide(), RiskDecision::Review);
    }

    // ---- Blacklist override ------------------------------------------------

    #[test]
    fn blacklist_forces_decline() {
        let mut ctx = base_ctx();
        ctx.is_blacklisted = true;
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert_eq!(a.decision, RiskDecision::Decline);
        assert!((a.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn blacklist_overrides_high_score() {
        let mut ctx = base_ctx();
        ctx.is_blacklisted = true;
        ctx.amount_minor_units = 10_000_00; // huge amount
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert_eq!(a.decision, RiskDecision::Decline);
    }

    // ---- Whitelist override ------------------------------------------------

    #[test]
    fn whitelist_forces_allow() {
        let mut ctx = base_ctx();
        ctx.is_whitelisted = true;
        ctx.amount_minor_units = 10_000_00; // huge amount
        ctx.recent_tx_count_from_ip = 100; // high velocity
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        assert_eq!(a.decision, RiskDecision::Allow);
        assert!(a.breakdown.whitelist_override);
    }

    // ---- Whitelist takes precedence over blacklist ------------------------

    #[test]
    fn whitelist_overrides_blacklist() {
        let mut ctx = base_ctx();
        ctx.is_blacklisted = true;
        ctx.is_whitelisted = true;
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &default_thresholds());

        // Whitelist is evaluated after blacklist; if both fire, whitelist
        // sets score to 0.0 and decision becomes Allow.
        assert_eq!(a.decision, RiskDecision::Allow);
    }

    // ---- Empty rules → score 0.0 ------------------------------------------

    #[test]
    fn evaluate_empty_rules() {
        let ctx = base_ctx();
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &[], &default_thresholds());
        assert_eq!(a.score, 0.0);
        assert_eq!(a.decision, RiskDecision::Allow);
    }

    // ---- Score stays in 0.0–1.0 range ------------------------------------

    #[test]
    fn score_always_clamped() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 10_000_00;
        ctx.recent_tx_count_from_ip = 100;
        ctx.recent_tx_count_from_card = 100;
        ctx.country_code = Some("XX".to_string());
        let mut t = default_thresholds();
        t.allowed_countries = vec!["AE".to_string()];
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &t);

        assert!(a.score >= 0.0);
        assert!(a.score <= 1.0);
    }

    // ---- Breakdown populated correctly ------------------------------------

    #[test]
    fn breakdown_reflects_triggered_rules() {
        let mut ctx = base_ctx();
        ctx.amount_minor_units = 800_00;
        ctx.recent_tx_count_from_ip = 20;
        ctx.country_code = Some("XX".to_string());
        let mut t = default_thresholds();
        t.allowed_countries = vec!["AE".to_string()];
        let mut a = RiskAssessment::new(ctx.payment_intent_id, ctx.operator_id);
        a.evaluate(&ctx, &all_rules(), &t);

        assert!(a.breakdown.amount_factor > 0.0);
        assert!(a.breakdown.velocity_factor > 0.0);
        assert!(a.breakdown.geo_factor > 0.0);
        assert!((a.breakdown.blacklist_factor).abs() < f64::EPSILON);
        assert!(!a.breakdown.whitelist_override);
    }
}

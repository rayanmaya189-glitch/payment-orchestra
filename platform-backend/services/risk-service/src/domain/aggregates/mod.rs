use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{RiskDecision, RiskFactor};

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub score: f64,
    pub decision: RiskDecision,
    pub factors: Vec<RiskFactor>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub velocity_score: f64,
    pub geo_score: f64,
    pub behavior_score: f64,
    pub is_whitelisted: bool,
    pub is_blacklisted: bool,
    pub created_at: DateTime<Utc>,
}

impl RiskAssessment {
    pub fn new(payment_intent_id: Uuid, operator_id: Uuid) -> Self {
        Self {
            assessment_id: Uuid::now_v7(), payment_intent_id, operator_id,
            score: 0.0, decision: RiskDecision::Allow, factors: Vec::new(),
            ip_address: None, user_agent: None,
            velocity_score: 0.0, geo_score: 0.0, behavior_score: 0.0,
            is_whitelisted: false, is_blacklisted: false, created_at: Utc::now(),
        }
    }

    pub fn evaluate(&mut self, rules: &[RiskRule]) {
        let mut total_score = 0.0;
        self.factors.clear();

        for rule in rules {
            if let Some(factor) = rule.evaluate(self) {
                total_score += factor.score;
                self.factors.push(factor);
            }
        }

        self.score = total_score.clamp(0.0, 100.0);
        self.decision = if self.is_blacklisted {
            RiskDecision::Decline
        } else if self.is_whitelisted {
            RiskDecision::Allow
        } else if self.score <= 30.0 {
            RiskDecision::Allow
        } else if self.score <= 60.0 {
            RiskDecision::Review
        } else {
            RiskDecision::Decline
        };
    }
}

pub struct RiskRule {
    pub name: String,
    pub evaluator: Box<dyn Fn(&RiskAssessment) -> Option<RiskFactor> + Send + Sync>,
}

impl RiskRule {
    pub fn new(name: &str, evaluator: impl Fn(&RiskAssessment) -> Option<RiskFactor> + Send + Sync + 'static) -> Self {
        Self { name: name.to_string(), evaluator: Box::new(evaluator) }
    }

    pub fn evaluate(&self, assessment: &RiskAssessment) -> Option<RiskFactor> {
        (self.evaluator)(assessment)
    }
}

pub fn default_rules() -> Vec<RiskRule> {
    vec![
        RiskRule::new("high_amount", |a| {
            None // Placeholder — real implementation checks amount > threshold
        }),
        RiskRule::new("velocity", |a| {
            None // Placeholder — real implementation checks transaction velocity
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_assessment() {
        let a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        assert_eq!(a.score, 0.0);
        assert_eq!(a.decision, RiskDecision::Allow);
    }

    #[test]
    fn test_evaluate_empty_rules() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.evaluate(&[]);
        assert_eq!(a.score, 0.0);
        assert_eq!(a.decision, RiskDecision::Allow);
    }

    #[test]
    fn test_blacklist_overrides() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.is_blacklisted = true;
        a.evaluate(&[]);
        assert_eq!(a.decision, RiskDecision::Decline);
    }

    #[test]
    fn test_whitelist_overrides() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7());
        a.is_whitelisted = true;
        a.score = 80.0;
        a.evaluate(&[]);
        assert_eq!(a.decision, RiskDecision::Allow);
    }
}

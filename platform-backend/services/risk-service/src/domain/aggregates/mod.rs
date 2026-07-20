use chrono::{DateTime, Utc}; use uuid::Uuid;
use crate::domain::value_objects::{RiskDecision, RiskFactor};
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub assessment_id: Uuid, pub operator_id: Uuid, pub payment_intent_id: Uuid,
    pub score: f64, pub decision: RiskDecision, pub factors: Vec<RiskFactor>,
    pub amount: Money, pub created_at: DateTime<Utc>,
}
impl RiskAssessment {
    pub fn new(operator_id: Uuid, payment_intent_id: Uuid, amount: Money) -> Self {
        Self { assessment_id: Uuid::now_v7(), operator_id, payment_intent_id, score: 0.0, decision: RiskDecision::Review, factors: Vec::new(), amount, created_at: Utc::now() }
    }
    pub fn evaluate(&mut self) {
        self.score = self.factors.iter().map(|f| f.score * f.weight).sum::<f64>() / self.factors.iter().map(|f| f.weight).sum::<f64>().max(1.0);
        self.decision = if self.score >= 0.8 { RiskDecision::Approve } else if self.score >= 0.5 { RiskDecision::Review } else { RiskDecision::Reject };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_assessment_is_review() {
        let a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7(), Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        assert_eq!(a.decision, RiskDecision::Review);
        assert_eq!(a.score, 0.0);
        assert!(a.factors.is_empty());
    }

    #[test]
    fn test_evaluate_approve_high_score() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7(), Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        a.factors.push(RiskFactor { factor: "amount".into(), score: 0.9, weight: 1.0, description: "Low amount".into() });
        a.evaluate();
        assert_eq!(a.decision, RiskDecision::Approve);
        assert!(a.score >= 0.8);
    }

    #[test]
    fn test_evaluate_review_medium_score() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7(), Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        a.factors.push(RiskFactor { factor: "amount".into(), score: 0.6, weight: 1.0, description: "Medium amount".into() });
        a.evaluate();
        assert_eq!(a.decision, RiskDecision::Review);
    }

    #[test]
    fn test_evaluate_reject_low_score() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7(), Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        a.factors.push(RiskFactor { factor: "amount".into(), score: 0.2, weight: 1.0, description: "High amount".into() });
        a.evaluate();
        assert_eq!(a.decision, RiskDecision::Reject);
    }

    #[test]
    fn test_evaluate_weighted_average() {
        let mut a = RiskAssessment::new(Uuid::now_v7(), Uuid::now_v7(), Money { amount_minor_units: 10000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        a.factors.push(RiskFactor { factor: "amount".into(), score: 0.9, weight: 2.0, description: "Low amount".into() });
        a.factors.push(RiskFactor { factor: "velocity".into(), score: 0.3, weight: 1.0, description: "High velocity".into() });
        a.evaluate();
        // (0.9*2 + 0.3*1) / (2+1) = 2.1/3 = 0.7
        assert!((a.score - 0.7).abs() < 0.01);
        assert_eq!(a.decision, RiskDecision::Review);
    }

    #[test]
    fn test_risk_decision_values() {
        assert_eq!(RiskDecision::Approve.as_str(), "approve");
        assert_eq!(RiskDecision::Review.as_str(), "review");
        assert_eq!(RiskDecision::Reject.as_str(), "reject");
    }
}

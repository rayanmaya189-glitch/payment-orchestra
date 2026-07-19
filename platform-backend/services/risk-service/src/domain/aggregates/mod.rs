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

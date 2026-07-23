//! Fraud & Risk Scoring domain events — BC-11

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskEvent {
    ScoreAssigned(RiskScoreAssigned),
    TransactionFlagged(RiskTransactionFlagged),
}

/// Event type string constants.
pub const EVENT_TYPE_SCORE_ASSIGNED: &str = "risk.score_assigned";
pub const EVENT_TYPE_FLAGGED: &str = "risk.transaction_flagged";

impl RiskEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::ScoreAssigned(_) => EVENT_TYPE_SCORE_ASSIGNED,
            Self::TransactionFlagged(_) => EVENT_TYPE_FLAGGED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScoreAssigned {
    pub risk_assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub risk_score: f64,
    pub risk_level: String,
    pub risk_factors: Vec<String>,
    pub rule_version: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskTransactionFlagged {
    pub risk_assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub risk_score: f64,
    pub risk_level: String,
    pub risk_factors: Vec<String>,
    pub occurred_at: DateTime<Utc>,
}

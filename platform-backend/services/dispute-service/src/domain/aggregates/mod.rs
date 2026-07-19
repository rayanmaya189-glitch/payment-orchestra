use chrono::{DateTime, Utc}; use uuid::Uuid;
use crate::domain::value_objects::{DisputeReason, DisputeStatus};
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct Dispute {
    pub dispute_id: Uuid, pub operator_id: Uuid, pub payment_intent_id: Uuid, pub status: DisputeStatus,
    pub reason: DisputeReason, pub amount: Money, pub evidence: Option<String>,
    pub created_at: DateTime<Utc>, pub resolved_at: Option<DateTime<Utc>>,
}
impl Dispute {
    pub fn new(operator_id: Uuid, payment_intent_id: Uuid, reason: DisputeReason, amount: Money) -> Self {
        Self { dispute_id: Uuid::now_v7(), operator_id, payment_intent_id, status: DisputeStatus::Open, reason, amount, evidence: None, created_at: Utc::now(), resolved_at: None }
    }
    pub fn can_submit_evidence(&self) -> bool { self.status == DisputeStatus::Open || self.status == DisputeStatus::UnderReview }
    pub fn can_resolve(&self) -> bool { self.status == DisputeStatus::UnderReview }
    pub fn can_close(&self) -> bool { self.status != DisputeStatus::Closed }
}

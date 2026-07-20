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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dispute_starts_open() {
        let d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), DisputeReason::Fraudulent, Money { amount_minor_units: 5000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        assert_eq!(d.status, DisputeStatus::Open);
        assert!(d.evidence.is_none());
    }

    #[test]
    fn test_can_submit_evidence_when_open() {
        let d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), DisputeReason::Fraudulent, Money { amount_minor_units: 5000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        assert!(d.can_submit_evidence());
    }

    #[test]
    fn test_cannot_resolve_when_open() {
        let d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), DisputeReason::Fraudulent, Money { amount_minor_units: 5000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        assert!(!d.can_resolve());
    }

    #[test]
    fn test_can_resolve_when_under_review() {
        let mut d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), DisputeReason::Fraudulent, Money { amount_minor_units: 5000, currency: shared_types::CurrencyCode::new("AED").unwrap() });
        d.status = DisputeStatus::UnderReview;
        assert!(d.can_resolve());
    }

    #[test]
    fn test_dispute_status_values() {
        assert_eq!(DisputeStatus::Open.as_str(), "open");
        assert_eq!(DisputeStatus::UnderReview.as_str(), "under_review");
        assert_eq!(DisputeStatus::Resolved.as_str(), "resolved");
        assert_eq!(DisputeStatus::Closed.as_str(), "closed");
    }
}

use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;
use crate::domain::value_objects::{DisputeStatus, DisputeDecision};

#[derive(Debug, Clone)]
pub struct Dispute {
    pub dispute_id: Uuid, pub payment_intent_id: Uuid, pub operator_id: Uuid,
    pub status: DisputeStatus, pub reason: String, pub reason_code: Option<String>,
    pub disputed_amount: Money, pub acquirer_reference: String, pub connector_id: String,
    pub acquirer_dispute_id: Option<String>, pub evidence: Option<serde_json::Value>,
    pub decision: Option<DisputeDecision>, pub decision_reason: Option<String>,
    pub opened_at: DateTime<Utc>, pub respond_by: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
}

impl Dispute {
    pub fn new(payment_intent_id: Uuid, operator_id: Uuid, reason: String, amount: Money, acquirer_ref: String, connector_id: String) -> Self {
        let now = Utc::now();
        Self { dispute_id: Uuid::now_v7(), payment_intent_id, operator_id, status: DisputeStatus::Opened,
            reason, reason_code: None, disputed_amount: amount, acquirer_reference: acquirer_ref,
            connector_id, acquirer_dispute_id: None, evidence: None, decision: None,
            decision_reason: None, opened_at: now, respond_by: Some(now + chrono::Duration::days(30)),
            resolved_at: None, created_at: now, updated_at: now }
    }

    pub fn submit_evidence(&mut self, evidence: serde_json::Value) {
        self.evidence = Some(evidence);
        self.status = DisputeStatus::EvidenceSubmitted;
        self.updated_at = Utc::now();
    }

    pub fn resolve(&mut self, decision: DisputeDecision, reason: &str) {
        self.decision = Some(decision);
        self.decision_reason = Some(reason.to_string());
        self.status = DisputeStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn aed(amount: i64) -> Money { Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() } }

    #[test]
    fn test_new_dispute() {
        let d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), "fraud".into(), aed(1000), "acq_123".into(), "ni".into());
        assert_eq!(d.status, DisputeStatus::Opened);
    }

    #[test]
    fn test_submit_evidence() {
        let mut d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), "fraud".into(), aed(1000), "acq_123".into(), "ni".into());
        d.submit_evidence(serde_json::json!({"docs": ["receipt.pdf"]}));
        assert_eq!(d.status, DisputeStatus::EvidenceSubmitted);
    }

    #[test]
    fn test_resolve() {
        let mut d = Dispute::new(Uuid::now_v7(), Uuid::now_v7(), "fraud".into(), aed(1000), "acq_123".into(), "ni".into());
        d.resolve(DisputeDecision::Won, "Evidence sufficient");
        assert_eq!(d.status, DisputeStatus::Resolved);
        assert!(d.resolved_at.is_some());
    }
}

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{KybStatus, KybDecision};

#[derive(Debug, Clone)]
pub struct KybCase {
    pub case_id: Uuid,
    pub operator_id: Uuid,
    pub status: KybStatus,
    pub assigned_officer: Option<Uuid>,
    pub risk_score: Option<f64>,
    pub decision: Option<KybDecision>,
    pub decision_reason: Option<String>,
    pub notes: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub documents: Vec<KybDocument>,
    pub uncommitted_events: Vec<KybEvent>,
}

#[derive(Debug, Clone)]
pub struct KybDocument {
    pub document_id: Uuid,
    pub kyb_case_id: Uuid,
    pub document_type: String,
    pub file_key: String,
    pub file_hash: String,
    pub verified: bool,
    pub uploaded_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum KybEvent {
    Submitted { operator_id: Uuid },
    Assigned { officer_id: Uuid },
    DocumentUploaded { document_type: String },
    DocumentVerified { document_id: Uuid },
    DecisionMade { decision: String, reason: String },
}

impl KybCase {
    pub fn new(operator_id: Uuid) -> Self {
        let now = Utc::now();
        let mut c = Self {
            case_id: Uuid::now_v7(),
            operator_id,
            status: KybStatus::Submitted,
            assigned_officer: None,
            risk_score: None,
            decision: None,
            decision_reason: None,
            notes: None,
            submitted_at: now,
            reviewed_at: None,
            decided_at: None,
            created_at: now,
            documents: Vec::new(),
            uncommitted_events: Vec::new(),
        };
        c.apply(KybEvent::Submitted { operator_id });
        c
    }

    pub fn apply(&mut self, event: KybEvent) {
        match &event {
            KybEvent::Assigned { officer_id } => {
                self.assigned_officer = Some(*officer_id);
                self.status = KybStatus::UnderReview;
                self.reviewed_at = Some(Utc::now());
            }
            KybEvent::DecisionMade { decision, .. } => {
                self.decision = Some(KybDecision::from_str(decision));
                self.status = match self.decision {
                    Some(KybDecision::Approved) => KybStatus::Approved,
                    Some(KybDecision::Rejected) => KybStatus::Rejected,
                    _ => KybStatus::UnderReview,
                };
                self.decided_at = Some(Utc::now());
            }
            _ => {}
        }
        self.uncommitted_events.push(event);
    }

    pub fn take_uncommitted_events(&mut self) -> Vec<KybEvent> {
        std::mem::take(&mut self.uncommitted_events)
    }

    pub fn assign_officer(&mut self, officer_id: Uuid) {
        self.apply(KybEvent::Assigned { officer_id });
    }

    pub fn decide(&mut self, decision: &str, reason: &str) {
        self.apply(KybEvent::DecisionMade {
            decision: decision.to_string(),
            reason: reason.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_kyb_case() {
        let c = KybCase::new(Uuid::now_v7());
        assert_eq!(c.status, KybStatus::Submitted);
        assert!(c.assigned_officer.is_none());
    }

    #[test]
    fn test_assign_officer() {
        let mut c = KybCase::new(Uuid::now_v7());
        c.assign_officer(Uuid::now_v7());
        assert_eq!(c.status, KybStatus::UnderReview);
        assert!(c.assigned_officer.is_some());
    }

    #[test]
    fn test_approve() {
        let mut c = KybCase::new(Uuid::now_v7());
        c.assign_officer(Uuid::now_v7());
        c.decide("approved", "All documents verified");
        assert_eq!(c.status, KybStatus::Approved);
        assert!(c.decided_at.is_some());
    }

    #[test]
    fn test_reject() {
        let mut c = KybCase::new(Uuid::now_v7());
        c.assign_officer(Uuid::now_v7());
        c.decide("rejected", "Invalid documents");
        assert_eq!(c.status, KybStatus::Rejected);
    }
}

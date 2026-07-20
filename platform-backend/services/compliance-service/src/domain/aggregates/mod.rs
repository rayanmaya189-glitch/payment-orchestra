use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{KybCaseStatus, KybDocumentType};

#[derive(Debug, Clone)]
pub struct KybCase {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub status: KybCaseStatus,
    pub assigned_compliance_officer: Option<Uuid>,
    pub documents: Vec<KybDocument>,
    pub risk_score: Option<f64>,
    pub decision: Option<KybDecision>,
    pub notes: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct KybDocument {
    pub id: Uuid,
    pub kyb_case_id: Uuid,
    pub document_type: KybDocumentType,
    pub file_key: String,
    pub file_hash: String,
    pub uploaded_at: DateTime<Utc>,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct KybDecision {
    pub decision: KybCaseStatus,
    pub reason: String,
    pub decided_by: Uuid,
    pub decided_at: DateTime<Utc>,
}

impl KybCase {
    pub fn new(operator_id: Uuid) -> Self {
        Self {
            id: Uuid::now_v7(),
            operator_id,
            status: KybCaseStatus::Submitted,
            assigned_compliance_officer: None,
            documents: Vec::new(),
            risk_score: None,
            decision: None,
            notes: None,
            submitted_at: Utc::now(),
            reviewed_at: None,
            decided_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn can_be_decided(&self) -> bool {
        matches!(
            self.status,
            KybCaseStatus::UnderReview | KybCaseStatus::DocumentsVerified
        )
    }

    pub fn all_documents_verified(&self) -> bool {
        !self.documents.is_empty() && self.documents.iter().all(|d| d.verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyb_case_new_starts_submitted() {
        let operator_id = Uuid::now_v7();
        let case = KybCase::new(operator_id);
        assert_eq!(case.status, KybCaseStatus::Submitted);
        assert_eq!(case.operator_id, operator_id);
        assert!(case.documents.is_empty());
        assert!(case.decision.is_none());
    }

    #[test]
    fn test_can_be_decided_only_in_review_or_verified() {
        let mut case = KybCase::new(Uuid::now_v7());

        // Submitted → cannot decide
        assert!(!case.can_be_decided());

        case.status = KybCaseStatus::UnderReview;
        assert!(case.can_be_decided());

        case.status = KybCaseStatus::DocumentsVerified;
        assert!(case.can_be_decided());

        case.status = KybCaseStatus::Approved;
        assert!(!case.can_be_decided());

        case.status = KybCaseStatus::Rejected;
        assert!(!case.can_be_decided());
    }

    #[test]
    fn test_all_documents_verified() {
        let mut case = KybCase::new(Uuid::now_v7());

        // No documents → false
        assert!(!case.all_documents_verified());

        // Add unverified document
        case.documents.push(KybDocument {
            id: Uuid::now_v7(),
            kyb_case_id: case.id,
            document_type: KybDocumentType::TradeLicense,
            file_key: "key1".to_string(),
            file_hash: "hash1".to_string(),
            uploaded_at: Utc::now(),
            verified: false,
            verified_at: None,
        });
        assert!(!case.all_documents_verified());

        // Verify it
        case.documents[0].verified = true;
        assert!(case.all_documents_verified());
    }

    #[test]
    fn test_kyb_document_types() {
        assert_eq!(KybDocumentType::TradeLicense.as_str(), "trade_license");
        assert_eq!(KybDocumentType::BoardResolution.as_str(), "board_resolution");
        assert_eq!(KybDocumentType::UboDeclaration.as_str(), "ubo_declaration");
    }

    #[test]
    fn test_kyb_case_status_values() {
        assert_eq!(KybCaseStatus::Submitted.as_str(), "submitted");
        assert_eq!(KybCaseStatus::UnderReview.as_str(), "under_review");
        assert_eq!(KybCaseStatus::Approved.as_str(), "approved");
        assert_eq!(KybCaseStatus::Rejected.as_str(), "rejected");
        assert_eq!(KybCaseStatus::Suspended.as_str(), "suspended");
    }
}

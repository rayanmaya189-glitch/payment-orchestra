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

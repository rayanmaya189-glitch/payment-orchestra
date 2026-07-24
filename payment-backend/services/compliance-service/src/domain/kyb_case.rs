use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ComplianceError;

/// A Know Your Business case for operator verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KybCase {
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub status: KybStatus,
    pub submitted_by: Uuid,
    pub document_ids: Vec<Uuid>,
    pub ocr_extracted_fields: Option<String>,
    pub partner_decision: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KybStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
}

impl KybStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "submitted" => Some(Self::Submitted),
            "under_review" => Some(Self::UnderReview),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Approved | Self::Rejected)
    }
}

impl KybCase {
    pub fn new(
        operator_id: Uuid,
        submitted_by: Uuid,
        document_ids: Vec<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            kyb_case_id: Uuid::now_v7(),
            operator_id,
            status: KybStatus::Submitted,
            submitted_by,
            document_ids,
            ocr_extracted_fields: None,
            partner_decision: None,
            rejection_reason: None,
            submitted_at: now,
            resolved_at: None,
            updated_at: now,
        }
    }

    pub fn approve(&mut self) -> Result<(), ComplianceError> {
        if self.status.is_terminal() {
            return Err(ComplianceError::KybCaseAlreadyResolved(self.kyb_case_id));
        }
        self.status = KybStatus::Approved;
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn reject(&mut self, reason: String) -> Result<(), ComplianceError> {
        if self.status.is_terminal() {
            return Err(ComplianceError::KybCaseAlreadyResolved(self.kyb_case_id));
        }
        self.status = KybStatus::Rejected;
        self.rejection_reason = Some(reason);
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
}

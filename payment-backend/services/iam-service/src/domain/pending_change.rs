use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::IamError;

/// A change that requires Maker/Checker approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub change_id: Uuid,
    pub change_type: String,
    pub maker_id: Uuid,
    pub checker_id: Option<Uuid>,
    pub payload: Vec<u8>,
    pub status: ChangeStatus,
    pub maker_note: Option<String>,
    pub checker_note: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl ChangeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

impl PendingChange {
    pub fn new(change_type: String, maker_id: Uuid, payload: Vec<u8>, maker_note: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            change_id: Uuid::now_v7(),
            change_type,
            maker_id,
            checker_id: None,
            payload,
            status: ChangeStatus::Pending,
            maker_note,
            checker_note: None,
            requested_at: now,
            reviewed_at: None,
            expires_at: now + chrono::Duration::hours(72),
        }
    }

    pub fn approve(&mut self, checker_id: Uuid, note: Option<String>) -> Result<(), IamError> {
        if self.status != ChangeStatus::Pending {
            return Err(IamError::ChangeNotPending);
        }
        if checker_id == self.maker_id {
            return Err(IamError::SelfApprovalNotAllowed);
        }
        self.checker_id = Some(checker_id);
        self.checker_note = note;
        self.status = ChangeStatus::Approved;
        self.reviewed_at = Some(Utc::now());
        Ok(())
    }

    pub fn reject(&mut self, checker_id: Uuid, note: Option<String>) -> Result<(), IamError> {
        if self.status != ChangeStatus::Pending {
            return Err(IamError::ChangeNotPending);
        }
        self.checker_id = Some(checker_id);
        self.checker_note = note;
        self.status = ChangeStatus::Rejected;
        self.reviewed_at = Some(Utc::now());
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

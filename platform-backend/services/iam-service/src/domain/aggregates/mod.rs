#![allow(dead_code)]
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Principal {
    pub id: Uuid,
    pub principal_type: PrincipalType,
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,
    pub mfa_enrolled: bool,
    pub mfa_method: Option<String>,
    pub status: PrincipalStatus,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalType {
    Human,
    ApiKey,
    Service,
}

impl PrincipalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::ApiKey => "api_key",
            Self::Service => "service",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "human" => Ok(Self::Human),
            "api_key" => Ok(Self::ApiKey),
            "service" => Ok(Self::Service),
            _ => Err("unknown principal type"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalStatus {
    Active,
    Suspended,
    Deleted,
}

impl PrincipalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "deleted" => Ok(Self::Deleted),
            _ => Err("unknown principal status"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingChange {
    pub change_id: Uuid,
    pub change_type: String,
    pub maker_id: Uuid,
    pub checker_id: Option<Uuid>,
    pub payload: Vec<u8>,
    pub status: PendingChangeStatus,
    pub maker_note: Option<String>,
    pub checker_note: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChangeStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl PendingChangeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            _ => Err("unknown pending change status"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Principal {
    pub fn is_locked(&self) -> bool {
        self.locked_until
            .map(|until| until > Utc::now())
            .unwrap_or(false)
    }

    pub fn record_failed_login(&mut self, lockout_15min: i32, lockout_1hr: i32, lockout_suspend: i32) {
        self.failed_login_attempts += 1;
        if self.failed_login_attempts == lockout_15min {
            self.locked_until = Some(Utc::now() + chrono::Duration::minutes(15));
        } else if self.failed_login_attempts == lockout_1hr {
            self.locked_until = Some(Utc::now() + chrono::Duration::hours(1));
        } else if self.failed_login_attempts >= lockout_suspend {
            self.status = PrincipalStatus::Suspended;
        }
    }

    pub fn record_successful_login(&mut self) {
        self.failed_login_attempts = 0;
        self.locked_until = None;
        self.last_login_at = Some(Utc::now());
    }
}

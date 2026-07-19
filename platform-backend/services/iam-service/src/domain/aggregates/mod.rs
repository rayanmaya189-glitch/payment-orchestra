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

    pub fn from_str(s: &str) -> Self {
        match s {
            "human" => Self::Human,
            "api_key" => Self::ApiKey,
            "service" => Self::Service,
            _ => Self::Human,
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

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "suspended" => Self::Suspended,
            "deleted" => Self::Deleted,
            _ => Self::Active,
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

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "expired" => Self::Expired,
            _ => Self::Pending,
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

    pub fn record_failed_login(&mut self) {
        self.failed_login_attempts += 1;
        match self.failed_login_attempts {
            5 => self.locked_until = Some(Utc::now() + chrono::Duration::minutes(15)),
            10 => self.locked_until = Some(Utc::now() + chrono::Duration::hours(1)),
            20 => self.status = PrincipalStatus::Suspended,
            _ => {}
        }
    }

    pub fn record_successful_login(&mut self) {
        self.failed_login_attempts = 0;
        self.locked_until = None;
        self.last_login_at = Some(Utc::now());
    }
}

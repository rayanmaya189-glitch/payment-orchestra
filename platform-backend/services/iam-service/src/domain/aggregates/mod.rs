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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_principal() -> Principal {
        Principal {
            id: Uuid::now_v7(),
            principal_type: PrincipalType::Human,
            email: Some("test@example.com".to_string()),
            password_hash: Some(vec![1, 2, 3]),
            mfa_enrolled: false,
            mfa_method: None,
            status: PrincipalStatus::Active,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            last_login_at: None,
        }
    }

    #[test]
    fn test_principal_not_locked_initially() {
        let p = make_principal();
        assert!(!p.is_locked());
    }

    #[test]
    fn test_lockout_after_5_failures() {
        let mut p = make_principal();
        for _ in 0..5 {
            p.record_failed_login(5, 10, 20);
        }
        assert!(p.is_locked());
        assert!(p.locked_until.is_some());
        // Still active, not suspended
        assert_eq!(p.status, PrincipalStatus::Active);
    }

    #[test]
    fn test_lockout_after_10_failures() {
        let mut p = make_principal();
        for _ in 0..10 {
            p.record_failed_login(5, 10, 20);
        }
        assert!(p.is_locked());
        // 10 failures = 1hr lockout (overrides 15min)
        let locked_until = p.locked_until.unwrap();
        let now = Utc::now();
        let diff = locked_until - now;
        assert!(diff.num_minutes() > 50 && diff.num_minutes() <= 60);
    }

    #[test]
    fn test_suspension_after_20_failures() {
        let mut p = make_principal();
        for _ in 0..20 {
            p.record_failed_login(5, 10, 20);
        }
        assert_eq!(p.status, PrincipalStatus::Suspended);
    }

    #[test]
    fn test_successful_login_resets_counters() {
        let mut p = make_principal();
        for _ in 0..4 {
            p.record_failed_login(5, 10, 20);
        }
        assert_eq!(p.failed_login_attempts, 4);
        assert!(!p.is_locked());

        p.record_successful_login();
        assert_eq!(p.failed_login_attempts, 0);
        assert!(p.locked_until.is_none());
        assert!(p.last_login_at.is_some());
    }

    #[test]
    fn test_principal_type_from_str() {
        assert_eq!(PrincipalType::from_str("human").unwrap(), PrincipalType::Human);
        assert_eq!(PrincipalType::from_str("api_key").unwrap(), PrincipalType::ApiKey);
        assert_eq!(PrincipalType::from_str("service").unwrap(), PrincipalType::Service);
        assert!(PrincipalType::from_str("unknown").is_err());
    }

    #[test]
    fn test_principal_status_from_str() {
        assert_eq!(PrincipalStatus::from_str("active").unwrap(), PrincipalStatus::Active);
        assert_eq!(PrincipalStatus::from_str("suspended").unwrap(), PrincipalStatus::Suspended);
        assert_eq!(PrincipalStatus::from_str("deleted").unwrap(), PrincipalStatus::Deleted);
        assert!(PrincipalStatus::from_str("unknown").is_err());
    }

    #[test]
    fn test_pending_change_status_from_str() {
        assert_eq!(PendingChangeStatus::from_str("pending").unwrap(), PendingChangeStatus::Pending);
        assert_eq!(PendingChangeStatus::from_str("approved").unwrap(), PendingChangeStatus::Approved);
        assert_eq!(PendingChangeStatus::from_str("rejected").unwrap(), PendingChangeStatus::Rejected);
        assert_eq!(PendingChangeStatus::from_str("expired").unwrap(), PendingChangeStatus::Expired);
        assert!(PendingChangeStatus::from_str("unknown").is_err());
    }

    #[test]
    fn test_lockout_with_custom_thresholds() {
        let mut p = make_principal();
        // Custom: 3 → 15min, 5 → 1hr, 8 → suspend
        for _ in 0..3 {
            p.record_failed_login(3, 5, 8);
        }
        assert!(p.is_locked());
        assert_eq!(p.status, PrincipalStatus::Active);

        // Reset
        p.record_successful_login();

        // 8 suspends
        for _ in 0..8 {
            p.record_failed_login(3, 5, 8);
        }
        assert_eq!(p.status, PrincipalStatus::Suspended);
    }
}

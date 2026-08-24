use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::AuthError;

/// Principal — a user, API key, or service that can authenticate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub id: Uuid,
    pub principal_type: PrincipalType,
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,
    pub mfa_enrolled: bool,
    pub mfa_method: Option<MfaMethod>,
    pub status: PrincipalStatus,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub operator_id: Option<Uuid>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "human" => Some(Self::Human),
            "api_key" => Some(Self::ApiKey),
            "service" => Some(Self::Service),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MfaMethod {
    WebAuthn,
    Totp,
}

impl MfaMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WebAuthn => "webauthn",
            Self::Totp => "totp",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "webauthn" => Some(Self::WebAuthn),
            "totp" => Some(Self::Totp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

impl Principal {
    #[allow(dead_code)]
    pub fn new_human(id: Uuid, email: String, password_hash: Vec<u8>) -> Self {
        let now = Utc::now();
        Self {
            id,
            principal_type: PrincipalType::Human,
            email: Some(email),
            password_hash: Some(password_hash),
            mfa_enrolled: false,
            mfa_method: None,
            status: PrincipalStatus::Active,
            roles: vec!["user".into()],
            permissions: vec![],
            operator_id: None,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            last_login_at: None,
            updated_at: now,
        }
    }

    /// Get effective permissions for this principal
    pub fn effective_permissions(&self) -> Vec<String> {
        let mut perms = self.permissions.clone();
        // Add role-based permissions
        for role in &self.roles {
            match role.as_str() {
                "admin" => {
                    perms.push("*".into());
                    perms.push("operator:create".into());
                    perms.push("operator:read".into());
                    perms.push("operator:update".into());
                    perms.push("operator:delete".into());
                    perms.push("payment:create".into());
                    perms.push("payment:read".into());
                    perms.push("payment:refund".into());
                    perms.push("connector:manage".into());
                    perms.push("settings:manage".into());
                }
                "operator" => {
                    perms.push("operator:read".into());
                    perms.push("operator:update".into());
                    perms.push("payment:create".into());
                    perms.push("payment:read".into());
                    perms.push("payment:refund".into());
                    perms.push("connector:manage".into());
                }
                "user" => {
                    perms.push("payment:create".into());
                    perms.push("payment:read".into());
                }
                "readonly" => {
                    perms.push("payment:read".into());
                    perms.push("operator:read".into());
                }
                _ => {}
            }
        }
        perms.sort();
        perms.dedup();
        perms
    }

    pub fn record_login_attempt(&mut self, success: bool) -> Result<(), AuthError> {
        if success {
            self.failed_login_attempts = 0;
            self.locked_until = None;
            self.last_login_at = Some(Utc::now());
            Ok(())
        } else {
            self.failed_login_attempts += 1;
            match self.failed_login_attempts {
                0..=4 => Ok(()),
                5..=9 => {
                    self.locked_until = Some(Utc::now() + chrono::Duration::minutes(15));
                    Err(AuthError::AccountLocked(std::time::Duration::from_secs(15 * 60)))
                }
                10..=19 => {
                    self.locked_until = Some(Utc::now() + chrono::Duration::hours(1));
                    Err(AuthError::AccountLocked(std::time::Duration::from_secs(60 * 60)))
                }
                _ => {
                    self.status = PrincipalStatus::Suspended;
                    Err(AuthError::AccountSuspended)
                }
            }
        }
    }

    pub fn is_locked(&self) -> bool {
        if let Some(until) = self.locked_until {
            Utc::now() < until
        } else {
            false
        }
    }

    pub fn can_authenticate(&self) -> Result<(), AuthError> {
        match self.status {
            PrincipalStatus::Suspended => Err(AuthError::AccountSuspended),
            PrincipalStatus::Deleted => Err(AuthError::AccountDeleted),
            PrincipalStatus::Active => {
                if self.is_locked() {
                    Err(AuthError::AccountLocked(
                        self.locked_until
                            .map(|t| (t - Utc::now()).to_std().unwrap_or_default())
                            .unwrap_or_default(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

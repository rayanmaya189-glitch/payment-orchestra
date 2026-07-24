//! Domain model for BC-02 Identity & Access Management.
//! Owns Principal aggregate, PendingChange aggregate (Maker/Checker),
//! MFA enrollments, API keys, and ABAC policy evaluation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Principal Aggregate ────────────────────────────────────────────────────

/// Principal — a user, API key, or service that can authenticate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub id: Uuid,
    pub principal_type: PrincipalType,
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>, // Argon2id hash
    pub mfa_enrolled: bool,
    pub mfa_method: Option<MfaMethod>,
    pub status: PrincipalStatus,
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
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            last_login_at: None,
            updated_at: now,
        }
    }

    pub fn record_login_attempt(&mut self, success: bool) -> Result<(), AuthError> {
        if success {
            self.failed_login_attempts = 0;
            self.locked_until = None;
            self.last_login_at = Some(Utc::now());
            Ok(())
        } else {
            self.failed_login_attempts += 1;
            // AUTH-007: 5 failed → 15min lockout; 10 → 1hr; 20 → suspension
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

// ─── PendingChange Aggregate (Maker/Checker) ────────────────────────────────

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
            expires_at: now + chrono::Duration::hours(72), // default 72h expiry
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

// ─── API Key ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,  // Argon2id hash of the key secret
    pub scopes: Vec<String>,
    pub status: ApiKeyStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl ApiKeyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    #[allow(dead_code)]
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "revoked" => Some(Self::Revoked),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

// ─── Auth Errors ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Account locked for {0:?}")]
    AccountLocked(std::time::Duration),
    #[error("Account suspended")]
    AccountSuspended,
    #[error("Account deleted")]
    AccountDeleted,
    #[error("MFA required")]
    MfaRequired,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
}

// ─── IAM Errors ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum IamError {
    #[error("Principal not found: {0}")]
    PrincipalNotFound(Uuid),
    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),
    #[error("Maker/Checker: change not pending")]
    ChangeNotPending,
    #[error("Maker/Checker: self-approval not allowed")]
    SelfApprovalNotAllowed,
    #[error("Maker/Checker: change expired")]
    ChangeExpired,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Duplicate API key name: {0}")]
    DuplicateApiKeyName(String),
    #[error(transparent)]
    Auth(#[from] AuthError),
}

impl From<IamError> for platform_error::PlatformError {
    fn from(e: IamError) -> Self {
        match e {
            IamError::PrincipalNotFound(id) => {
                platform_error::PlatformError::NotFound { resource: "principal", id }
            }
            IamError::AuthorizationDenied(ref msg) => {
                platform_error::PlatformError::AuthorizationDenied(msg.clone())
            }
            IamError::ChangeNotPending | IamError::SelfApprovalNotAllowed | IamError::ChangeExpired => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "change".into(),
                        reason: e.to_string(),
                    },
                )
            }
            IamError::InvalidRequest(ref msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "request".into(),
                        reason: msg.clone(),
                    },
                )
            }
            IamError::DuplicateApiKeyName(_msg) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            IamError::Auth(AuthError::InvalidCredentials) => {
                platform_error::PlatformError::AuthorizationDenied("Invalid credentials".into())
            }
            IamError::Auth(AuthError::AccountLocked(_)) => {
                platform_error::PlatformError::AuthorizationDenied("Account locked".into())
            }
            IamError::Auth(AuthError::AccountSuspended) => {
                platform_error::PlatformError::AuthorizationDenied("Account suspended".into())
            }
            IamError::Auth(AuthError::AccountDeleted) => {
                platform_error::PlatformError::AuthorizationDenied("Account deleted".into())
            }
            IamError::Auth(AuthError::MfaRequired) => {
                platform_error::PlatformError::AuthorizationDenied("MFA required".into())
            }
            IamError::Auth(AuthError::TokenExpired | AuthError::InvalidToken) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "token".into(),
                        reason: e.to_string(),
                    },
                )
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principal_lockout_5_failures() {
        let hash = vec![0u8; 32];
        let mut p = Principal::new_human(Uuid::now_v7(), "test@test.com".into(), hash);
        
        for _ in 0..5 {
            let _ = p.record_login_attempt(false);
        }
        assert!(p.is_locked());
    }

    #[test]
    fn test_principal_lockout_resets_on_success() {
        let hash = vec![0u8; 32];
        let mut p = Principal::new_human(Uuid::now_v7(), "test@test.com".into(), hash);
        
        for _ in 0..4 {
            let _ = p.record_login_attempt(false);
        }
        assert!(p.record_login_attempt(true).is_ok());
        assert!(!p.is_locked());
        assert_eq!(p.failed_login_attempts, 0);
    }

    #[test]
    fn test_pending_change_self_approval_rejected() {
        let maker_id = Uuid::now_v7();
        let mut change = PendingChange::new(
            "update_gateway".into(),
            maker_id,
            vec![],
            None,
        );
        let result = change.approve(maker_id, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_pending_change_approve_success() {
        let maker_id = Uuid::now_v7();
        let checker_id = Uuid::now_v7();
        let mut change = PendingChange::new(
            "update_gateway".into(),
            maker_id,
            vec![],
            None,
        );
        assert!(change.approve(checker_id, None).is_ok());
        assert_eq!(change.status, ChangeStatus::Approved);
    }

    #[test]
    fn test_pending_change_double_approve_fails() {
        let maker_id = Uuid::now_v7();
        let checker_id = Uuid::now_v7();
        let mut change = PendingChange::new(
            "update_gateway".into(),
            maker_id,
            vec![],
            None,
        );
        change.approve(checker_id, None).unwrap();
        assert!(change.approve(Uuid::now_v7(), None).is_err());
    }
}

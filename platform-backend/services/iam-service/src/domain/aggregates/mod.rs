use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{Email, MfaMethod, PrincipalRole, PrincipalStatus};

/// Principal aggregate root — represents a user or service account.
#[derive(Debug, Clone)]
pub struct Principal {
    pub principal_id: Uuid,
    pub principal_type: PrincipalType,
    pub email: Option<Email>,
    pub password_hash: Option<Vec<u8>>,
    pub mfa_enrolled: bool,
    pub mfa_method: Option<MfaMethod>,
    pub mfa_secret: Option<String>,
    pub status: PrincipalStatus,
    pub role: PrincipalRole,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub uncommitted_events: Vec<PrincipalEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalType {
    User,
    ApiClient,
    System,
}

impl PrincipalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::ApiClient => "api_client",
            Self::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "api_client" => Self::ApiClient,
            "system" => Self::System,
            _ => Self::User,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrincipalEvent {
    Created {
        email: Option<String>,
        principal_type: String,
    },
    PasswordSet,
    LoginSucceeded {
        ip_address: String,
        user_agent: String,
    },
    LoginFailed {
        reason: String,
        ip_address: String,
    },
    AccountLocked {
        locked_until: DateTime<Utc>,
        reason: String,
    },
    AccountUnlocked,
    MfaEnrolled {
        method: String,
    },
    MfaDisabled,
    PasswordChanged,
    ApiKeyCreated {
        api_key_id: Uuid,
        name: String,
    },
    ApiKeyRevoked {
        api_key_id: Uuid,
    },
}

impl Principal {
    pub fn new_user(email: Email, password_hash: Vec<u8>) -> Self {
        let now = Utc::now();
        let mut principal = Self {
            principal_id: Uuid::now_v7(),
            principal_type: PrincipalType::User,
            email: Some(email.clone()),
            password_hash: Some(password_hash),
            mfa_enrolled: false,
            mfa_method: None,
            mfa_secret: None,
            status: PrincipalStatus::Active,
            role: PrincipalRole::OperatorAdmin,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            last_login_at: None,
            uncommitted_events: Vec::new(),
        };
        principal.apply(PrincipalEvent::Created {
            email: Some(email.to_string()),
            principal_type: "user".to_string(),
        });
        principal
    }

    pub fn new_api_client() -> Self {
        let now = Utc::now();
        let mut principal = Self {
            principal_id: Uuid::now_v7(),
            principal_type: PrincipalType::ApiClient,
            email: None,
            password_hash: None,
            mfa_enrolled: false,
            mfa_method: None,
            mfa_secret: None,
            status: PrincipalStatus::Active,
            role: PrincipalRole::ApiClient,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            last_login_at: None,
            uncommitted_events: Vec::new(),
        };
        principal.apply(PrincipalEvent::Created {
            email: None,
            principal_type: "api_client".to_string(),
        });
        principal
    }

    pub fn apply(&mut self, event: PrincipalEvent) {
        match &event {
            PrincipalEvent::LoginSucceeded { .. } => {
                self.failed_login_attempts = 0;
                self.last_login_at = Some(Utc::now());
            }
            PrincipalEvent::LoginFailed { .. } => {
                self.failed_login_attempts += 1;
            }
            PrincipalEvent::AccountLocked { locked_until, .. } => {
                self.locked_until = Some(*locked_until);
                self.status = PrincipalStatus::Locked;
            }
            PrincipalEvent::AccountUnlocked => {
                self.locked_until = None;
                self.failed_login_attempts = 0;
                self.status = PrincipalStatus::Active;
            }
            PrincipalEvent::MfaEnrolled { method } => {
                self.mfa_enrolled = true;
                self.mfa_method = Some(MfaMethod::from_str(method));
            }
            PrincipalEvent::MfaDisabled => {
                self.mfa_enrolled = false;
                self.mfa_method = None;
                self.mfa_secret = None;
            }
            PrincipalEvent::PasswordChanged => {
                self.failed_login_attempts = 0;
            }
            _ => {}
        }
        self.uncommitted_events.push(event);
    }

    pub fn take_uncommitted_events(&mut self) -> Vec<PrincipalEvent> {
        std::mem::take(&mut self.uncommitted_events)
    }

    pub fn is_locked(&self) -> bool {
        match self.locked_until {
            Some(locked_until) => Utc::now() < locked_until,
            None => false,
        }
    }

    pub fn should_lock(&self, lockout_threshold: i32) -> bool {
        self.failed_login_attempts >= lockout_threshold
    }

    pub fn record_successful_login(&mut self, ip_address: &str, user_agent: &str) {
        self.apply(PrincipalEvent::LoginSucceeded {
            ip_address: ip_address.to_string(),
            user_agent: user_agent.to_string(),
        });
    }

    pub fn record_failed_login(&mut self, reason: &str, ip_address: &str) {
        self.apply(PrincipalEvent::LoginFailed {
            reason: reason.to_string(),
            ip_address: ip_address.to_string(),
        });
    }

    pub fn lock_account(&mut self, duration_minutes: i64, reason: &str) {
        let locked_until = Utc::now() + chrono::Duration::minutes(duration_minutes);
        self.apply(PrincipalEvent::AccountLocked {
            locked_until,
            reason: reason.to_string(),
        });
    }

    pub fn unlock_account(&mut self) {
        self.apply(PrincipalEvent::AccountUnlocked);
    }
}

/// API Key aggregate.
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ApiKey {
    pub fn new(
        principal_id: Uuid,
        name: String,
        key_hash: Vec<u8>,
        key_prefix: String,
        scopes: Vec<String>,
        expires_in_days: u32,
    ) -> Self {
        Self {
            api_key_id: Uuid::now_v7(),
            principal_id,
            name,
            key_hash,
            key_prefix,
            scopes,
            acquirer_link_ids: None,
            expires_at: Utc::now() + chrono::Duration::days(expires_in_days as i64),
            revoked_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.revoked_at.is_none() && Utc::now() < self.expires_at
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }
}

/// Refresh token aggregate.
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub token_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub client_fingerprint: String,
    pub status: RefreshTokenStatus,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshTokenStatus {
    Active,
    Revoked,
}

impl RefreshTokenStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

impl RefreshToken {
    pub fn new(principal_id: Uuid, role: String, client_fingerprint: String) -> Self {
        Self {
            token_id: Uuid::now_v7(),
            principal_id,
            role,
            client_fingerprint,
            status: RefreshTokenStatus::Active,
            created_at: Utc::now(),
            revoked_at: None,
            revoke_reason: None,
        }
    }

    pub fn revoke(&mut self, reason: &str) {
        self.status = RefreshTokenStatus::Revoked;
        self.revoked_at = Some(Utc::now());
        self.revoke_reason = Some(reason.to_string());
    }
}

/// ABAC policy rule.
#[derive(Debug, Clone)]
pub struct AbacPolicy {
    pub policy_id: Uuid,
    pub principal_id: Uuid,
    pub resource_type: String,
    pub action: String,
    pub effect: AbacEffect,
    pub conditions: Vec<AbacCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbacEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct AbacCondition {
    pub attribute: String,
    pub operator: AbacOperator,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbacOperator {
    Equals,
    NotEquals,
    In,
    Contains,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_email() -> Email {
        Email::new("test@example.com").unwrap()
    }

    #[test]
    fn test_new_user_principal() {
        let p = Principal::new_user(test_email(), vec![1, 2, 3]);
        assert_eq!(p.principal_type, PrincipalType::User);
        assert_eq!(p.status, PrincipalStatus::Active);
        assert_eq!(p.failed_login_attempts, 0);
        assert!(!p.is_locked());
        assert_eq!(p.uncommitted_events.len(), 1);
    }

    #[test]
    fn test_new_api_client() {
        let p = Principal::new_api_client();
        assert_eq!(p.principal_type, PrincipalType::ApiClient);
        assert!(p.email.is_none());
    }

    #[test]
    fn test_successful_login_resets_counter() {
        let mut p = Principal::new_user(test_email(), vec![1, 2, 3]);
        p.failed_login_attempts = 3;
        p.record_successful_login("1.2.3.4", "Mozilla/5.0");
        assert_eq!(p.failed_login_attempts, 0);
        assert!(p.last_login_at.is_some());
    }

    #[test]
    fn test_failed_login_increments_counter() {
        let mut p = Principal::new_user(test_email(), vec![1, 2, 3]);
        p.record_failed_login("bad password", "1.2.3.4");
        assert_eq!(p.failed_login_attempts, 1);
    }

    #[test]
    fn test_lock_account() {
        let mut p = Principal::new_user(test_email(), vec![1, 2, 3]);
        p.lock_account(15, "Too many failures");
        assert!(p.is_locked());
        assert_eq!(p.status, PrincipalStatus::Locked);
    }

    #[test]
    fn test_unlock_account() {
        let mut p = Principal::new_user(test_email(), vec![1, 2, 3]);
        p.lock_account(15, "Too many failures");
        p.unlock_account();
        assert!(!p.is_locked());
        assert_eq!(p.status, PrincipalStatus::Active);
    }

    #[test]
    fn test_should_lock() {
        let mut p = Principal::new_user(test_email(), vec![1, 2, 3]);
        assert!(!p.should_lock(5));
        p.failed_login_attempts = 5;
        assert!(p.should_lock(5));
    }

    #[test]
    fn test_api_key_valid() {
        let key = ApiKey::new(
            Uuid::now_v7(),
            "test".to_string(),
            vec![1, 2, 3],
            "pk_test".to_string(),
            vec!["read".to_string()],
            90,
        );
        assert!(key.is_valid());
    }

    #[test]
    fn test_api_key_revoked() {
        let mut key = ApiKey::new(
            Uuid::now_v7(),
            "test".to_string(),
            vec![1, 2, 3],
            "pk_test".to_string(),
            vec!["read".to_string()],
            90,
        );
        key.revoke();
        assert!(!key.is_valid());
    }

    #[test]
    fn test_refresh_token_revoked() {
        let mut token = RefreshToken::new(
            Uuid::now_v7(),
            "operator_admin".to_string(),
            "fp_123".to_string(),
        );
        assert_eq!(token.status, RefreshTokenStatus::Active);
        token.revoke("logout");
        assert_eq!(token.status, RefreshTokenStatus::Revoked);
    }
}

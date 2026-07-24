//! Domain model for BC-02 Identity & Access Management.
//! Owns Principal aggregate, PendingChange aggregate (Maker/Checker),
//! MFA enrollments, API keys, and ABAC policy evaluation.
//!
//! Each domain concept has its own file within this module.

// Sub-modules — one file per concept
pub mod api_key;
pub mod error;
pub mod pending_change;
pub mod principal;

// Re-export all types for convenience
pub use api_key::{ApiKey, ApiKeyStatus};
pub use error::{AuthError, IamError};
pub use pending_change::{ChangeStatus, PendingChange};
pub use principal::{MfaMethod, Principal, PrincipalStatus, PrincipalType};

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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

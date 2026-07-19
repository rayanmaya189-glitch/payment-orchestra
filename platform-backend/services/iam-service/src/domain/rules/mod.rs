#![allow(dead_code)]
use platform_error::PlatformError;

use crate::domain::aggregates::Principal;

/// AUTH-007: 5 failed -> 15min lockout; 10 -> 1hr; 20 -> suspension
pub fn assert_not_locked(principal: &Principal) -> Result<(), PlatformError> {
    if principal.is_locked() {
        return Err(PlatformError::AuthorizationDenied(format!(
            "Account {} is locked until {:?}",
            principal.id, principal.locked_until
        )));
    }
    Ok(())
}

/// AUTH-011: WebAuthn minimum for Admin/Finance roles
pub fn assert_mfa_required(role: &str, mfa_enrolled: bool) -> Result<(), PlatformError> {
    matches!(role, "admin" | "finance")
        .then_some(())
        .filter(|_| mfa_enrolled)
        .ok_or_else(|| {
            PlatformError::AuthorizationDenied(format!(
                "Role {} requires MFA enrollment",
                role
            ))
        })
}

/// Maker/Checker: self-approval rejected
pub fn assert_maker_checker_distinct(maker_id: uuid::Uuid, checker_id: uuid::Uuid) -> Result<(), PlatformError> {
    if maker_id == checker_id {
        return Err(PlatformError::AuthorizationDenied(
            "Maker cannot approve their own change".into(),
        ));
    }
    Ok(())
}

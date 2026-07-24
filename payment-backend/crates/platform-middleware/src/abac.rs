//! ABAC (Attribute-Based Access Control) authorization middleware.
//!
//! Permission strings follow the format: `<resource>:<action>`
//! Examples: `payment_intent:create`, `payment_intent:read`, `admin.*`
//!
//! Wildcard patterns are supported:
//! - `*` — superuser, grants all permissions
//! - `payment_intent:*` — all actions on payment_intents
//! - `admin.*` — admin-level access (equivalent to all permissions)

use uuid::Uuid;
use platform_error::PlatformError;
use shared_types::auth_context::AuthContext;

/// Check if a principal has the required permission.
///
/// Uses the AuthContext's resolved permission set (pre-loaded from the principal's roles).
///
/// # Arguments
/// * `auth_ctx` - The authenticated principal's context (from JWT or API key validation)
/// * `required_permission` - The permission string to check (e.g., `"payment_intent:create"`)
///
/// # Returns
/// * `Ok(true)` if permitted
/// * `Err(PlatformError::AuthorizationDenied(...))` if not permitted
pub fn check_permission(auth_ctx: &AuthContext, required_permission: &str) -> Result<bool, PlatformError> {
    if auth_ctx.has_permission(required_permission) {
        Ok(true)
    } else {
        Err(PlatformError::AuthorizationDenied(
            format!("Missing required permission: {}", required_permission)
        ))
    }
}

/// Check if a principal has all of the required permissions.
pub fn check_all_permissions(auth_ctx: &AuthContext, required_permissions: &[&str]) -> Result<bool, PlatformError> {
    for perm in required_permissions {
        check_permission(auth_ctx, perm)?;
    }
    Ok(true)
}

/// Check if a principal has any of the required permissions.
pub fn check_any_permission(auth_ctx: &AuthContext, required_permissions: &[&str]) -> Result<bool, PlatformError> {
    for perm in required_permissions {
        if auth_ctx.has_permission(perm) {
            return Ok(true);
        }
    }
    Err(PlatformError::AuthorizationDenied(
        format!("Missing any of the required permissions: {:?}", required_permissions)
    ))
}

/// Check resource ownership — verifies the principal owns the resource.
///
/// For operator-scoped resources, checks that the resource's operator_id
/// matches the authenticated principal's operator_id.
pub fn check_resource_ownership(auth_ctx: &AuthContext, resource_operator_id: Option<Uuid>) -> Result<bool, PlatformError> {
    // Admins can access any resource
    if auth_ctx.is_admin() {
        return Ok(true);
    }

    match (auth_ctx.operator_id, resource_operator_id) {
        (Some(auth_op), Some(res_op)) if auth_op == res_op => Ok(true),
        (Some(_), Some(_)) => Err(PlatformError::AuthorizationDenied(
            "Resource does not belong to the authenticated operator".into()
        )),
        (Some(_), None) => Err(PlatformError::AuthorizationDenied(
            "Resource ownership cannot be verified".into()
        )),
        (None, _) => {
            // Service principals with admin access can proceed
            if auth_ctx.principal_type == "service" {
                Ok(true)
            } else {
                Err(PlatformError::AuthorizationDenied(
                    "Principal has no operator context".into()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn admin_context() -> AuthContext {
        AuthContext {
            principal_id: Uuid::now_v7(),
            principal_type: "human".into(),
            operator_id: Some(Uuid::now_v7()),
            permissions: vec!["admin.*".into()],
            session_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    fn operator_context(permissions: Vec<String>) -> AuthContext {
        AuthContext {
            principal_id: Uuid::now_v7(),
            principal_type: "human".into(),
            operator_id: Some(Uuid::now_v7()),
            permissions,
            session_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    fn service_context() -> AuthContext {
        AuthContext {
            principal_id: Uuid::now_v7(),
            principal_type: "service".into(),
            operator_id: None,
            permissions: vec![],
            session_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let ctx = admin_context();
        assert!(check_permission(&ctx, "payment_intent:create").unwrap());
        assert!(check_permission(&ctx, "payment_intent:delete").unwrap());
        assert!(check_permission(&ctx, "anything:at_all").unwrap());
    }

    #[test]
    fn test_operator_with_specific_permission() {
        let ctx = operator_context(vec!["payment_intent:create".into()]);
        assert!(check_permission(&ctx, "payment_intent:create").unwrap());
        assert!(check_permission(&ctx, "payment_intent:read").is_err());
    }

    #[test]
    fn test_wildcard_resource_permission() {
        let ctx = operator_context(vec!["payment_intent:*".into()]);
        assert!(check_permission(&ctx, "payment_intent:create").unwrap());
        assert!(check_permission(&ctx, "payment_intent:delete").unwrap());
        assert!(check_permission(&ctx, "invoice:read").is_err());
    }

    #[test]
    fn test_superuser_wildcard() {
        let ctx = operator_context(vec!["*".into()]);
        assert!(check_permission(&ctx, "anything").unwrap());
        assert!(check_permission(&ctx, "foo:bar:baz").unwrap());
    }

    #[test]
    fn test_check_all_permissions() {
        let ctx = operator_context(vec!["a:1".into(), "b:2".into()]);
        assert!(check_all_permissions(&ctx, &["a:1", "b:2"]).is_ok());
        assert!(check_all_permissions(&ctx, &["a:1", "missing"]).is_err());
    }

    #[test]
    fn test_check_any_permission() {
        let ctx = operator_context(vec!["a:1".into()]);
        assert!(check_any_permission(&ctx, &["a:1", "b:2"]).is_ok());
        assert!(check_any_permission(&ctx, &["x:9", "y:0"]).is_err());
    }

    #[test]
    fn test_ownership_admin_bypass() {
        let ctx = admin_context();
        let other_op = Uuid::now_v7();
        assert!(check_resource_ownership(&ctx, Some(other_op)).unwrap());
    }

    #[test]
    fn test_ownership_match() {
        let op_id = Uuid::now_v7();
        let ctx = operator_context(vec![]);
        let mut ctx_with_op = ctx;
        ctx_with_op.operator_id = Some(op_id);
        assert!(check_resource_ownership(&ctx_with_op, Some(op_id)).unwrap());
    }

    #[test]
    fn test_ownership_mismatch() {
        let op_id = Uuid::now_v7();
        let other_op = Uuid::now_v7();
        let ctx = operator_context(vec![]);
        let mut ctx_with_op = ctx;
        ctx_with_op.operator_id = Some(op_id);
        assert!(check_resource_ownership(&ctx_with_op, Some(other_op)).is_err());
    }

    #[test]
    fn test_service_principal_ownership_bypass() {
        let ctx = service_context();
        assert!(check_resource_ownership(&ctx, Some(Uuid::now_v7())).unwrap());
    }
}

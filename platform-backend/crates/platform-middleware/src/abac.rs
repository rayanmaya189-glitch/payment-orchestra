/// Attribute-Based Access Control (ABAC) enforcement.
///
/// Evaluates policies at the command handler level per SRS ABAC-001 through ABAC-008.
use uuid::Uuid;
use platform_error::PlatformError;

/// ABAC policy evaluation context.
#[derive(Debug, Clone)]
pub struct AbacContext {
    pub principal_id: Uuid,
    pub role: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<Uuid>,
    pub amount: Option<i64>,
    pub ip_address: Option<String>,
    pub operator_id: Option<Uuid>,
}

/// Evaluate ABAC policy for a given context.
///
/// Returns Ok(()) if allowed, Err if denied.
pub fn evaluate_policy(ctx: &AbacContext) -> Result<(), PlatformError> {
    // ABAC-004: Maker/Checker segregation — not enforced here, checked at command level
    // ABAC-005: Checker eligibility scoped by role

    match ctx.role.as_str() {
        "platform_admin" => Ok(()), // Platform admins can do everything
        "operator_admin" => evaluate_operator_admin(ctx),
        "compliance_officer" => evaluate_compliance_officer(ctx),
        "api_client" => evaluate_api_client(ctx),
        "read_only" => evaluate_read_only(ctx),
        _ => Err(PlatformError::AuthorizationDenied(
            format!("Unknown role: {}", ctx.role)
        )),
    }
}

fn evaluate_operator_admin(ctx: &AbacContext) -> Result<(), PlatformError> {
    match ctx.action.as_str() {
        "read" => Ok(()),
        "create" | "update" => {
            // Cannot modify operator or compliance resources
            if matches!(ctx.resource.as_str(), "operator" | "compliance") {
                return Err(PlatformError::AuthorizationDenied(
                    "Operator admin cannot modify operator or compliance resources".to_string()
                ));
            }
            Ok(())
        }
        "delete" => Err(PlatformError::AuthorizationDenied(
            "Operator admin cannot delete resources".to_string()
        )),
        _ => Err(PlatformError::AuthorizationDenied(
            format!("Operator admin cannot perform action: {}", ctx.action)
        )),
    }
}

fn evaluate_compliance_officer(ctx: &AbacContext) -> Result<(), PlatformError> {
    match ctx.action.as_str() {
        "read" => Ok(()), // Can read everything
        "update" => {
            // Can only update kyb_case
            if ctx.resource != "kyb_case" {
                return Err(PlatformError::AuthorizationDenied(
                    "Compliance officer can only update KYB cases".to_string()
                ));
            }
            Ok(())
        }
        "create" => {
            // Can only create KYB decisions
            if ctx.resource != "kyb_decision" {
                return Err(PlatformError::AuthorizationDenied(
                    "Compliance officer can only create KYB decisions".to_string()
                ));
            }
            Ok(())
        }
        _ => Err(PlatformError::AuthorizationDenied(
            format!("Compliance officer cannot perform action: {}", ctx.action)
        )),
    }
}

fn evaluate_api_client(ctx: &AbacContext) -> Result<(), PlatformError> {
    match ctx.action.as_str() {
        "read" | "create" => Ok(()),
        "update" | "delete" => Err(PlatformError::AuthorizationDenied(
            "API client cannot update or delete resources".to_string()
        )),
        _ => Err(PlatformError::AuthorizationDenied(
            format!("API client cannot perform action: {}", ctx.action)
        )),
    }
}

fn evaluate_read_only(ctx: &AbacContext) -> Result<(), PlatformError> {
    match ctx.action.as_str() {
        "read" => Ok(()),
        _ => Err(PlatformError::AuthorizationDenied(
            "Read-only role can only read resources".to_string()
        )),
    }
}

/// Check if the principal can approve this pending change (Maker/Checker segregation).
///
/// SRS ABAC-004: No principal may serve as both Maker and Checker.
pub fn check_maker_checker(
    maker_id: Uuid,
    checker_id: Uuid,
    _pending_change_id: Uuid,
) -> Result<(), PlatformError> {
    if maker_id == checker_id {
        return Err(PlatformError::AuthorizationDenied(
            "Maker and Checker must be different principals (SRS ABAC-004)".to_string()
        ));
    }
    Ok(())
}

/// Check amount threshold for dual-control (SRS ABAC-001).
///
/// Returns true if second approver is required.
pub fn requires_dual_control(amount: i64, threshold: i64) -> bool {
    amount >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(role: &str, action: &str, resource: &str) -> AbacContext {
        AbacContext {
            principal_id: Uuid::now_v7(),
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        }
    }

    #[test]
    fn test_platform_admin_can_do_anything() {
        let ctx = make_ctx("platform_admin", "delete", "operator");
        assert!(evaluate_policy(&ctx).is_ok());
    }

    #[test]
    fn test_operator_admin_cannot_delete() {
        let ctx = make_ctx("operator_admin", "delete", "payment");
        assert!(evaluate_policy(&ctx).is_err());
    }

    #[test]
    fn test_read_only_can_only_read() {
        let ctx = make_ctx("read_only", "create", "payment");
        assert!(evaluate_policy(&ctx).is_err());
    }

    #[test]
    fn test_maker_checker_segregation() {
        let maker = Uuid::now_v7();
        let checker = Uuid::now_v7();
        assert!(check_maker_checker(maker, checker, Uuid::now_v7()).is_ok());
        assert!(check_maker_checker(maker, maker, Uuid::now_v7()).is_err());
    }

    #[test]
    fn test_dual_control_threshold() {
        assert!(!requires_dual_control(49999, 50000));
        assert!(requires_dual_control(50000, 50000));
        assert!(requires_dual_control(100000, 50000));
    }
}

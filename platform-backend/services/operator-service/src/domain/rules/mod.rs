use platform_error::PlatformError;

use crate::domain::aggregates::Operator;

/// BR-001-1: Cannot process live transactions until KYB status = Approved
pub fn assert_kyb_approved(operator: &Operator) -> Result<(), PlatformError> {
    if !operator.can_process_live_transactions() {
        return Err(PlatformError::AuthorizationDenied(format!(
            "Operator {} KYB not approved, current status: {}",
            operator.id, operator.status
        )));
    }
    Ok(())
}

/// BR-001-2: Sandbox access available immediately post email-verification
pub fn assert_sandbox_access(operator: &Operator) -> Result<(), PlatformError> {
    use crate::domain::value_objects::OperatorStatus;
    if operator.status == OperatorStatus::Pending {
        return Err(PlatformError::AuthorizationDenied(format!(
            "Operator {} email not verified",
            operator.id
        )));
    }
    Ok(())
}

/// Check if operator can perform action
pub fn assert_operator_active(operator: &Operator) -> Result<(), PlatformError> {
    use crate::domain::value_objects::OperatorStatus;
    match operator.status {
        OperatorStatus::Suspended | OperatorStatus::ExpiredUnverified => {
            Err(PlatformError::AuthorizationDenied(format!(
                "Operator {} is {}",
                operator.id, operator.status
            )))
        }
        _ => Ok(()),
    }
}

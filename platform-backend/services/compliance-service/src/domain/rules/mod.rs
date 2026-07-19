use platform_error::PlatformError;

use crate::domain::aggregates::KybCase;

/// KYB cases require all documents verified before decision
pub fn assert_documents_verified(kyb_case: &KybCase) -> Result<(), PlatformError> {
    if !kyb_case.all_documents_verified() {
        return Err(PlatformError::Validation(
            platform_error::ValidationError::MissingField(
                "All documents must be verified before decision".into(),
            ),
        ));
    }
    Ok(())
}

/// KYB case must be in a decidable state
pub fn assert_can_decide(kyb_case: &KybCase) -> Result<(), PlatformError> {
    if !kyb_case.can_be_decided() {
        return Err(PlatformError::Validation(
            platform_error::ValidationError::InvalidStateTransition {
                from: kyb_case.status.as_str().to_string(),
                command: "Decide".to_string(),
            },
        ));
    }
    Ok(())
}

/// Only compliance officers can review KYB cases
pub fn assert_compliance_officer(officer_id: Option<uuid::Uuid>) -> Result<(), PlatformError> {
    if officer_id.is_none() {
        return Err(PlatformError::AuthorizationDenied(
            "No compliance officer assigned".into(),
        ));
    }
    Ok(())
}

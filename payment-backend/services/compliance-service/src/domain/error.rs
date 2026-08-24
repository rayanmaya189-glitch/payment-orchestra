use uuid::Uuid;

/// Domain errors for merchant compliance operations.
#[derive(Debug, thiserror::Error)]
pub enum ComplianceError {
    #[error("KYB case not found: {0}")]
    KybCaseNotFound(Uuid),

    #[error("KYB case already resolved: {0}")]
    KybCaseAlreadyResolved(Uuid),

    #[error("At least one document required")]
    KybNoDocuments,

    #[error("AML alert not found: {0}")]
    AmlAlertNotFound(Uuid),

    #[error("AML alert already reviewed: {0}")]
    AmlAlertAlreadyReviewed(Uuid),

    #[error("SAR generation failed: {0}")]
    SarGenerationFailed(String),

    #[error("Partner API unavailable: {0}")]
    PartnerApiUnavailable(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<ComplianceError> for platform_error::PlatformError {
    fn from(e: ComplianceError) -> Self {
        match e {
            ComplianceError::KybCaseNotFound(id) | ComplianceError::AmlAlertNotFound(id) => {
                platform_error::PlatformError::NotFound { resource: "compliance", id }
            }
            ComplianceError::KybCaseAlreadyResolved(_) | ComplianceError::AmlAlertAlreadyReviewed(_) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            ComplianceError::KybNoDocuments | ComplianceError::InvalidRequest(_) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "request".into(),
                        reason: e.to_string(),
                    },
                )
            }
            ComplianceError::SarGenerationFailed(ref msg) => {
                platform_error::PlatformError::Internal(msg.clone())
            }
            ComplianceError::PartnerApiUnavailable(ref msg) => {
                platform_error::PlatformError::Unavailable(msg.clone())
            }
        }
    }
}

use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum OnboardingError {
    #[error("Onboarding request not found: {0}")]
    NotFound(Uuid),
    #[error("Invalid status transition")]
    InvalidTransition,
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
    #[error("Invalid field value: {0}")]
    InvalidFieldValue(String),
    #[error("Connector not found: {0}")]
    ConnectorNotFound(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Duplicate credentials: already in use")]
    DuplicateCredentials,
}

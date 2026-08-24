use uuid::Uuid;

/// Authentication-related errors.
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

/// IAM domain errors.
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

use uuid::Uuid;
use crate::error_code::InternalErrorCode;

#[derive(Debug, Clone, thiserror::Error)]
pub enum PlatformError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Not found: {resource} {id}")]
    NotFound { resource: &'static str, id: Uuid },

    #[error("Conflict: {0}")]
    Conflict(#[from] ConflictError),

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Rate limited")]
    RateLimited { retry_after_ms: u64 },

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },
}

impl PlatformError {
    pub fn internal_error_code(&self) -> InternalErrorCode {
        match self {
            Self::Validation(_) | Self::NotFound { .. } => InternalErrorCode::ValidationError,
            Self::Conflict(_) => InternalErrorCode::PermanentFailure,
            Self::AuthorizationDenied(_) => InternalErrorCode::AuthorizationDenied,
            Self::RateLimited { .. } => InternalErrorCode::RateLimited,
            Self::Unavailable(_) => InternalErrorCode::Unavailable,
            Self::Internal(_) | Self::ExternalService { .. } => InternalErrorCode::TransientFailure,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid currency code")]
    InvalidCurrencyCode,
    #[error("Negative amount")]
    NegativeAmount,
    #[error("Currency mismatch")]
    CurrencyMismatch,
    #[error("Amount overflow")]
    AmountOverflow,
    #[error("Invalid idempotency key")]
    InvalidIdempotencyKey,
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    #[error("Invalid state transition: {from_state} -> {command}")]
    InvalidStateTransition { from_state: String, command: String },
    #[error("Invalid value: {field}: {reason}")]
    InvalidValue { field: String, reason: String },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConflictError {
    #[error("Duplicate idempotency key with different payload")]
    IdempotencyKeyConflict,
    #[error("Optimistic concurrency violation: expected sequence {expected}, actual {actual}")]
    ConcurrencyViolation { expected: i64, actual: i64 },
    #[error("Payment intent already captured")]
    AlreadyCaptured,
    #[error("Payment intent fully refunded")]
    FullyRefunded,
    #[error("Duplicate order invoice")]
    DuplicateOrderInvoice,
}

pub type Result<T> = std::result::Result<T, PlatformError>;

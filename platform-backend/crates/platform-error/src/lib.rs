use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum PlatformError {
    #[error("Validation error: {0}")]
    Validation(ValidationError),

    #[error("Not found: {resource} {id}")]
    NotFound { resource: String, id: Uuid },

    #[error("Conflict: {0}")]
    Conflict(ConflictError),

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Service unavailable: {0}")]
    Unavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
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
    MissingField(String),
    #[error("Invalid state transition: {from} -> {command}")]
    InvalidStateTransition { from: String, command: String },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConflictError {
    #[error("Duplicate idempotency key with different payload")]
    IdempotencyKeyConflict,
    #[error("Optimistic concurrency violation")]
    ConcurrencyViolation,
    #[error("Payment intent already captured")]
    AlreadyCaptured,
    #[error("Payment intent fully refunded")]
    FullyRefunded,
    #[error("Duplicate order invoice")]
    DuplicateOrderInvoice,
}

impl From<ValidationError> for PlatformError {
    fn from(e: ValidationError) -> Self {
        PlatformError::Validation(e)
    }
}

impl From<ConflictError> for PlatformError {
    fn from(e: ConflictError) -> Self {
        PlatformError::Conflict(e)
    }
}

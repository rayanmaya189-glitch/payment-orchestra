use uuid::Uuid;

/// Domain errors for the payment orchestration engine.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OrchestrationError {
    #[error("PaymentIntent not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: i64, actual: i64 },

    #[error("Idempotency conflict: key {0} used with different payload")]
    IdempotencyConflict(String),

    #[error("No eligible route found")]
    NoEligibleRoute,

    #[error("Routing policy not found")]
    RoutingPolicyNotFound,

    #[error("All acquirers declined")]
    AllAcquirersDeclined,

    #[error("Payment method token not found or inactive")]
    PaymentMethodTokenInvalid,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl OrchestrationError {
    /// Get the error code for this error.
    pub fn error_code(&self) -> String {
        match self {
            Self::NotFound(_) => "PAYMENT_INTENT_NOT_FOUND".to_string(),
            Self::InvalidStateTransition(code) => code.clone(),
            Self::Validation(_) => "VALIDATION_ERROR".to_string(),
            Self::InvariantViolation(_) => "INVARIANT_VIOLATION".to_string(),
            Self::ConcurrencyConflict { .. } => "CONCURRENCY_CONFLICT".to_string(),
            Self::IdempotencyConflict(_) => "IDEMPOTENCY_CONFLICT".to_string(),
            Self::NoEligibleRoute => "NO_ELIGIBLE_ROUTE".to_string(),
            Self::RoutingPolicyNotFound => "ROUTING_POLICY_NOT_FOUND".to_string(),
            Self::AllAcquirersDeclined => "ALL_ACQUIRERS_DECLINED".to_string(),
            Self::PaymentMethodTokenInvalid => "PAYMENT_METHOD_TOKEN_INVALID".to_string(),
            Self::DatabaseError(_) => "DATABASE_ERROR".to_string(),
        }
    }

    /// Get the HTTP status code for this error.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::InvalidStateTransition(_) => 422,
            Self::Validation(_) => 400,
            Self::InvariantViolation(_) => 500,
            Self::ConcurrencyConflict { .. } => 409,
            Self::IdempotencyConflict(_) => 409,
            Self::NoEligibleRoute => 422,
            Self::RoutingPolicyNotFound => 404,
            Self::AllAcquirersDeclined => 422,
            Self::PaymentMethodTokenInvalid => 422,
            Self::DatabaseError(_) => 500,
        }
    }
}

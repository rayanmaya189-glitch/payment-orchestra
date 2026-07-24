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

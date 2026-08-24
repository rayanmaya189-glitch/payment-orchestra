//! Subscription error types.

use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Subscription not found: {0}")]
    NotFound(Uuid),
    #[error("Subscription already cancelled")]
    AlreadyCancelled,
    #[error("Cannot cancel subscription during renewal")]
    CannotCancelDuringRenewal,
    #[error("Invalid subscription plan amount: must be positive")]
    InvalidPlanAmount,
    #[error("Invalid subscription status transition")]
    InvalidTransition,
    #[error("Dunning retries exhausted")]
    DunningExhausted,
    #[error("Billing cycle not found")]
    BillingCycleNotFound,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

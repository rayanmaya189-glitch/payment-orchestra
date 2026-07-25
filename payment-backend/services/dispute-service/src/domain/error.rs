//! Dispute domain errors.

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Error)]
pub enum DisputeError {
    #[error("Chargeback case not found: {0}")]
    NotFound(Uuid),
    #[error("Chargeback already resolved")]
    AlreadyResolved,
    #[error("Payment intent must be in captured state")]
    PaymentIntentNotCaptured,
    #[error("Invalid status transition")]
    InvalidTransition,
    #[error("Invalid chargeback amount")]
    InvalidAmount,
    #[error("Invalid representment evidence: missing required fields")]
    InvalidRepresentmentEvidence,
    #[error("Representment deadline has passed")]
    RepresentmentDeadlinePassed,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

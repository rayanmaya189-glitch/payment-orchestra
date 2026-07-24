//! Error types for reconciliation-service domain.

use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReconciliationError {
    #[error("Settlement batch not found: {0}")]
    NotFound(Uuid),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Duplicate batch: checksum {0} already ingested")]
    DuplicateBatch(String),

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Ledger imbalance detected for transaction {0}")]
    LedgerImbalance(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

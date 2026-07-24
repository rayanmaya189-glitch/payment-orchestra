//! Saga error types — BC-17

use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum SagaError {
    #[error("Saga not found: {0}")]
    NotFound(Uuid),
    #[error("Saga already completed")]
    AlreadyCompleted,
    #[error("Invalid saga status transition")]
    InvalidTransition,
    #[error("Invalid step transition")]
    InvalidStepTransition,
    #[error("No steps defined for saga")]
    NoStepsDefined,
    #[error("Step not found")]
    StepNotFound,
    #[error("Saga step execution failed: {0}")]
    StepFailed(String),
    #[error("Compensation failed after retries: {0}")]
    CompensationFailed(String),
    #[error("Saga timed out")]
    Timeout,
}

//! SagaStep — a single step within a saga.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub step_id: Uuid,
    pub step_name: String,
    pub action: String,                // e.g., "authorize", "capture", "void"
    pub compensation_action: String,   // e.g., "void", "refund"
    pub status: StepStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Executing,
    Succeeded,
    Failed,
    Compensated,
}

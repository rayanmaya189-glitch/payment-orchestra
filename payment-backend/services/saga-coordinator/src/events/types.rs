//! Saga Coordinator domain events — BC-17

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SagaEvent {
    Started(SagaStarted),
    StepCompleted(SagaStepCompleted),
    StepFailed(SagaStepFailed),
    Completed(SagaCompleted),
    Compensated(SagaCompensated),
    CompensationFailed(SagaCompensationFailed),
}

pub const EVENT_TYPE_STARTED: &str = "saga.started";
pub const EVENT_TYPE_STEP_COMPLETED: &str = "saga.step_completed";
pub const EVENT_TYPE_STEP_FAILED: &str = "saga.step_failed";
pub const EVENT_TYPE_COMPLETED: &str = "saga.completed";
pub const EVENT_TYPE_COMPENSATED: &str = "saga.compensated";
pub const EVENT_TYPE_COMPENSATION_FAILED: &str = "saga.compensation_failed";

impl SagaEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Started(_) => EVENT_TYPE_STARTED,
            Self::StepCompleted(_) => EVENT_TYPE_STEP_COMPLETED,
            Self::StepFailed(_) => EVENT_TYPE_STEP_FAILED,
            Self::Completed(_) => EVENT_TYPE_COMPLETED,
            Self::Compensated(_) => EVENT_TYPE_COMPENSATED,
            Self::CompensationFailed(_) => EVENT_TYPE_COMPENSATION_FAILED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStarted {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub aggregate_id: Uuid,
    pub total_steps: usize,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStepCompleted {
    pub saga_id: Uuid,
    pub step_name: String,
    pub step_index: usize,
    pub output: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStepFailed {
    pub saga_id: Uuid,
    pub step_name: String,
    pub step_index: usize,
    pub error: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCompleted {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub steps_completed: usize,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCompensated {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub steps_compensated: usize,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCompensationFailed {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub error: String,
    pub occurred_at: DateTime<Utc>,
}

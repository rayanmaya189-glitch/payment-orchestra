//! Command types for BC-17 Saga Coordinator

use uuid::Uuid;
use crate::domain::*;

/// Start a new saga execution.
pub struct StartSagaCommand {
    pub saga_type: SagaType,
    pub aggregate_id: Uuid,
    pub steps: Vec<SagaStep>,
}

/// Begin execution of a saga (Created → Running).
pub struct BeginSagaCommand {
    pub saga_id: Uuid,
}

/// Mark a step as started.
pub struct StartStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
}

/// Mark a step as completed.
pub struct CompleteStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
    pub output: String,
}

/// Mark a step as failed (triggers compensation).
pub struct FailStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
    pub error: String,
}

/// Compensate the next succeeded step (reverse order).
pub struct CompensateNextCommand {
    pub saga_id: Uuid,
}

/// Mark saga as completed.
pub struct CompleteSagaCommand {
    pub saga_id: Uuid,
}

/// Mark saga as failed (non-compensated).
pub struct FailSagaCommand {
    pub saga_id: Uuid,
    pub error: String,
}

use uuid::Uuid;
use crate::domain::aggregates::SagaStepDef;

/// Command to start a new saga.
#[derive(Debug, Clone)]
pub struct StartSagaCommand {
    pub saga_type: String,
    pub steps: Vec<SagaStepDef>,
    pub payload: serde_json::Value,
    pub started_by: Option<Uuid>,
}

/// Command to advance the current step of a saga.
#[derive(Debug, Clone)]
pub struct AdvanceSagaCommand {
    pub saga_id: Uuid,
    pub step_result: Option<serde_json::Value>,
}

/// Command to report a step failure.
#[derive(Debug, Clone)]
pub struct FailSagaCommand {
    pub saga_id: Uuid,
    pub error: String,
    pub error_code: Option<String>,
}

/// Command to compensate a saga after failure.
#[derive(Debug, Clone)]
pub struct CompensateSagaCommand {
    pub saga_id: Uuid,
    pub reason: Option<String>,
}

/// Command to record that a single step's compensation succeeded.
#[derive(Debug, Clone)]
pub struct MarkStepCompensatedCommand {
    pub saga_id: Uuid,
    pub step_number: u32,
}

/// Command to retrieve a saga by ID.
#[derive(Debug, Clone)]
pub struct GetSagaQuery {
    pub saga_id: Uuid,
}

/// Command to list sagas with optional filters.
#[derive(Debug, Clone)]
pub struct ListSagasQuery {
    pub saga_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

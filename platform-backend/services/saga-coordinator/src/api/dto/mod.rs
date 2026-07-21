use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct StartSagaRequest {
    pub saga_type: String,
    pub steps: Option<Vec<SagaStepDefRequest>>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SagaStepDefRequest {
    pub name: String,
    pub service: String,
    pub action: String,
    pub compensation_action: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartSagaResponse {
    pub saga_id: Uuid,
    pub status: String,
    pub total_steps: u32,
}

#[derive(Debug, Serialize)]
pub struct SagaDetailResponse {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub steps: Vec<SagaStepResponse>,
    pub payload: serde_json::Value,
    pub compensation_data: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SagaStepResponse {
    pub step_number: u32,
    pub name: String,
    pub service: String,
    pub action: String,
    pub compensation_action: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FailSagaRequest {
    pub error: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompensateSagaRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarkStepCompensatedRequest {
    pub step_number: u32,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl From<&crate::domain::aggregates::SagaInstance> for SagaDetailResponse {
    fn from(saga: &crate::domain::aggregates::SagaInstance) -> Self {
        Self {
            saga_id: saga.saga_id,
            saga_type: saga.saga_type.clone(),
            status: saga.status.as_str().to_string(),
            current_step: saga.current_step,
            total_steps: saga.total_steps,
            steps: saga.steps.iter().map(SagaStepResponse::from).collect(),
            payload: saga.payload.clone(),
            compensation_data: saga.compensation_data.clone(),
            created_at: saga.created_at.to_rfc3339(),
            updated_at: saga.updated_at.to_rfc3339(),
            completed_at: saga.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

impl From<&crate::domain::aggregates::SagaStep> for SagaStepResponse {
    fn from(step: &crate::domain::aggregates::SagaStep) -> Self {
        Self {
            step_number: step.step_number,
            name: step.name.clone(),
            service: step.service.clone(),
            action: step.action.clone(),
            compensation_action: step.compensation_action.clone(),
            status: step.status.as_str().to_string(),
            error: step.error.clone(),
        }
    }
}

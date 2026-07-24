//! gRPC service implementation for saga-coordinator (BC-17).
//! Translates between protobuf types and domain types for saga lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{FailSagaCommand, StartStepCommand};
use crate::domain::{SagaError, StepStatus};
use crate::api::SagaApi;

use platform_proto::common::Timestamp;
use platform_proto::saga::saga_service_server::SagaService;
use platform_proto::saga::*;

pub struct SagaGrpcService {
    api: SagaApi,
}

impl SagaGrpcService {
    pub fn new(api: SagaApi) -> Self {
        Self { api }
    }
}

#[tonic::async_trait]
impl SagaService for SagaGrpcService {
    async fn get_saga_instance(
        &self,
        request: Request<GetSagaInstanceRequest>,
    ) -> Result<Response<SagaInstanceView>, Status> {
        let req = request.into_inner();
        let saga_id = parse_uuid(&req.saga_id, "saga_id")?;

        match self.api.get_saga(saga_id).await {
            Ok(saga) => Ok(Response::new(saga_to_view(saga))),
            Err(e) => Err(saga_error_to_status(e)),
        }
    }

    async fn list_saga_instances(
        &self,
        request: Request<ListSagaInstancesRequest>,
    ) -> Result<Response<ListSagaInstancesResponse>, Status> {
        let req = request.into_inner();
        let aggregate_id = parse_uuid(&req.aggregate_id, "aggregate_id")?;

        let status_filter = if req.status_filter.is_empty() {
            None
        } else {
            Some(req.status_filter)
        };

        match self.api.find_by_aggregate(aggregate_id).await {
            Ok(sagas) => {
                let filtered: Vec<SagaInstanceView> = sagas
                    .into_iter()
                    .filter(|s| {
                        if let Some(ref filter) = status_filter {
                            s.status.to_string() == *filter
                        } else {
                            true
                        }
                    })
                    .map(saga_to_view)
                    .collect();

                Ok(Response::new(ListSagaInstancesResponse { sagas: filtered }))
            }
            Err(e) => Err(saga_error_to_status(e)),
        }
    }

    async fn retry_saga_step(
        &self,
        request: Request<RetrySagaStepRequest>,
    ) -> Result<Response<RetrySagaStepResponse>, Status> {
        let req = request.into_inner();
        let saga_id = parse_uuid(&req.saga_id, "saga_id")?;

        // Get the saga to find the step index by name
        let saga = self.api.get_saga(saga_id).await.map_err(saga_error_to_status)?;

        let step_idx = saga.steps.iter().position(|s| s.step_name == req.step_name)
            .ok_or_else(|| Status::not_found(format!("Step '{}' not found in saga", req.step_name)))?;

        // Re-start the failed step
        let cmd = StartStepCommand { saga_id, step_index: step_idx };
        self.api.start_step(cmd).await.map_err(saga_error_to_status)?;

        Ok(Response::new(RetrySagaStepResponse {
            accepted: true,
            status: "running".to_string(),
        }))
    }

    async fn compensate_saga(
        &self,
        request: Request<CompensateSagaRequest>,
    ) -> Result<Response<CompensateSagaResponse>, Status> {
        let req = request.into_inner();
        let saga_id = parse_uuid(&req.saga_id, "saga_id")?;

        // Mark saga as failed to trigger compensation flow
        let fail_cmd = FailSagaCommand {
            saga_id,
            error: req.reason,
        };
        self.api.fail_saga(fail_cmd).await.map_err(saga_error_to_status)?;

        Ok(Response::new(CompensateSagaResponse {
            accepted: true,
            status: "failed".to_string(),
        }))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn saga_to_view(saga: crate::domain::SagaInstance) -> SagaInstanceView {
    let steps: Vec<SagaStepResult> = saga.steps.iter().map(|s| {
        SagaStepResult {
            step_name: s.step_name.clone(),
            status: step_status_to_string(&s.status),
            result_json: s.output.clone().unwrap_or_default(),
            executed_at: s.started_at.map(|dt| Timestamp {
                unix_ms: dt.timestamp_millis(),
            }).or_else(|| s.completed_at.map(|dt| Timestamp {
                unix_ms: dt.timestamp_millis(),
            })),
            retry_count: 0,
        }
    }).collect();

    SagaInstanceView {
        saga_id: saga.saga_id.to_string(),
        saga_type: saga.saga_type.to_string(),
        status: saga.status.to_string(),
        aggregate_id: saga.aggregate_id.to_string(),
        steps,
        started_at: Some(Timestamp {
            unix_ms: saga.created_at.timestamp_millis(),
        }),
        completed_at: None,
    }
}

fn step_status_to_string(status: &StepStatus) -> String {
    match status {
        StepStatus::Pending => "pending".to_string(),
        StepStatus::Executing => "executing".to_string(),
        StepStatus::Succeeded => "succeeded".to_string(),
        StepStatus::Failed => "failed".to_string(),
        StepStatus::Compensated => "compensated".to_string(),
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn saga_error_to_status(e: SagaError) -> Status {
    match e {
        SagaError::NotFound(id) => Status::not_found(format!("Saga not found: {}", id)),
        SagaError::AlreadyCompleted => Status::failed_precondition("Saga already completed"),
        SagaError::InvalidTransition => Status::failed_precondition("Invalid saga status transition"),
        SagaError::InvalidStepTransition => Status::failed_precondition("Invalid step transition"),
        SagaError::NoStepsDefined => Status::invalid_argument("No steps defined for saga"),
        SagaError::StepNotFound => Status::not_found("Step not found"),
        SagaError::StepFailed(msg) => Status::internal(format!("Step failed: {}", msg)),
        SagaError::CompensationFailed(msg) => Status::internal(format!("Compensation failed: {}", msg)),
        SagaError::Timeout => Status::deadline_exceeded("Saga timed out"),
    }
}

impl From<SagaError> for Status {
    fn from(e: SagaError) -> Self {
        saga_error_to_status(e)
    }
}

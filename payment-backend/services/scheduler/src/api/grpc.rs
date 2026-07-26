//! gRPC service implementation for the scheduler management API.
//! Translates between protobuf types and domain types.
//! Provides operational RPCs for inspecting and managing scheduled jobs.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{
    PauseJob, ResumeJob,
};
use crate::api::SchedulerApi;
use crate::domain::SchedulerError;

use platform_proto::scheduler::scheduler_service_server::SchedulerService;
use platform_proto::scheduler::*;

pub struct SchedulerGrpcService {
    api: SchedulerApi,
}

impl SchedulerGrpcService {
    pub fn new(api: SchedulerApi) -> Self {
        Self { api }
    }
}

#[tonic::async_trait]
impl SchedulerService for SchedulerGrpcService {
    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<JobView>, Status> {
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id, "job_id")?;

        match self.api.get_job(job_id).await {
            Ok(job) => Ok(Response::new(job_to_view(job))),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn list_jobs(
        &self,
        request: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        let req = request.into_inner();

        let jobs = if req.service_name.is_empty() {
            self.api.list_jobs().await
        } else {
            self.api.list_jobs_by_service(&req.service_name).await
        };

        match jobs {
            Ok(jobs) => Ok(Response::new(ListJobsResponse {
                jobs: jobs.into_iter().map(job_to_view).collect(),
            })),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn list_active_jobs(
        &self,
        _request: Request<ListActiveJobsRequest>,
    ) -> Result<Response<ListActiveJobsResponse>, Status> {
        match self.api.list_active_jobs().await {
            Ok(jobs) => Ok(Response::new(ListActiveJobsResponse {
                jobs: jobs.into_iter().map(job_to_view).collect(),
            })),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn list_executions(
        &self,
        request: Request<ListExecutionsRequest>,
    ) -> Result<Response<ListExecutionsResponse>, Status> {
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id, "job_id")?;

        match self.api.list_executions(job_id).await {
            Ok(executions) => Ok(Response::new(ListExecutionsResponse {
                executions: executions.into_iter().map(execution_to_view).collect(),
            })),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn pause_job(
        &self,
        request: Request<PauseJobRequest>,
    ) -> Result<Response<JobView>, Status> {
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id, "job_id")?;

        match self.api.pause_job(PauseJob { job_id }).await {
            Ok(job) => Ok(Response::new(job_to_view(job))),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn resume_job(
        &self,
        request: Request<ResumeJobRequest>,
    ) -> Result<Response<JobView>, Status> {
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id, "job_id")?;

        match self.api.resume_job(ResumeJob { job_id }).await {
            Ok(job) => Ok(Response::new(job_to_view(job))),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }

    async fn trigger_job(
        &self,
        request: Request<TriggerJobRequest>,
    ) -> Result<Response<JobView>, Status> {
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id, "job_id")?;

        // Triggering a job returns the current view; the actual execution
        // will be picked up by the scheduler loop on its next tick.
        match self.api.get_job(job_id).await {
            Ok(job) => Ok(Response::new(job_to_view(job))),
            Err(e) => Err(scheduler_error_to_status(e)),
        }
    }
}

// ─── View Helpers ───────────────────────────────────────────────────────────

fn job_to_view(job: crate::domain::ScheduledJob) -> JobView {
    let (schedule_type, schedule_interval) = match job.schedule {
        crate::domain::JobSchedule::Every { interval_seconds } => {
            ("every".to_string(), format!("{}s", interval_seconds))
        }
        crate::domain::JobSchedule::Cron { expression } => {
            ("cron".to_string(), expression)
        }
        crate::domain::JobSchedule::OnceAt { run_at } => {
            ("once_at".to_string(), run_at.to_rfc3339())
        }
    };

    JobView {
        job_id: job.job_id.to_string(),
        job_key: job.job_key,
        service_name: job.service_name,
        description: job.description,
        schedule_type,
        schedule_interval,
        job_type: job.job_type.to_string(),
        status: format!("{:?}", job.status).to_lowercase(),
        last_run_at: job.last_run_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        last_run_success: job.last_result.as_ref().map(|r| r.success).unwrap_or(false),
        last_error: job.last_result.as_ref()
            .and_then(|r| r.error_message.clone())
            .unwrap_or_default(),
    }
}

fn execution_to_view(exec: crate::domain::JobExecution) -> ExecutionView {
    ExecutionView {
        execution_id: exec.execution_id.to_string(),
        job_id: exec.job_id.to_string(),
        job_key: exec.job_key,
        status: format!("{:?}", exec.status).to_lowercase(),
        started_at: exec.started_at.to_rfc3339(),
        completed_at: exec.completed_at.map(|t| t.to_rfc3339()),
        duration_ms: exec.duration_ms,
        success: exec.result.as_ref().map(|r| r.success),
        error_message: exec.result.as_ref()
            .and_then(|r| r.error_message.clone()),
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn scheduler_error_to_status(e: SchedulerError) -> Status {
    match e {
        SchedulerError::JobNotFound(id) => Status::not_found(format!("Job not found: {}", id)),
        SchedulerError::JobKeyConflict(ref key) => Status::already_exists(format!("Job key already exists: {}", key)),
        SchedulerError::InvalidSchedule(ref msg) => Status::invalid_argument(format!("Invalid schedule: {}", msg)),
        SchedulerError::ExecutionFailed(ref msg) => Status::internal(format!("Execution failed: {}", msg)),
        SchedulerError::JobDisabled(ref key) => Status::failed_precondition(format!("Job is disabled: {}", key)),
        SchedulerError::NotLeader(ref key) => Status::failed_precondition(format!("Not the leader for job: {}", key)),
        SchedulerError::DatabaseError(ref msg) => Status::internal(format!("Database error: {}", msg)),
    }
}

impl From<SchedulerError> for Status {
    fn from(e: SchedulerError) -> Self {
        scheduler_error_to_status(e)
    }
}

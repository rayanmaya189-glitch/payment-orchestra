//! Scheduler Service public API

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct SchedulerApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl SchedulerApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self { command_handler: ch, query_handler: qh }
    }

    pub async fn register_job(&self, cmd: RegisterJob) -> Result<ScheduledJob, SchedulerError> {
        self.command_handler.register_job(cmd).await
    }
    pub async fn execute_job(&self, cmd: ExecuteJob) -> Result<JobExecution, SchedulerError> {
        self.command_handler.execute_job(cmd).await
    }
    pub async fn pause_job(&self, cmd: PauseJob) -> Result<ScheduledJob, SchedulerError> {
        self.command_handler.pause_job(cmd).await
    }
    pub async fn resume_job(&self, cmd: ResumeJob) -> Result<ScheduledJob, SchedulerError> {
        self.command_handler.resume_job(cmd).await
    }
    pub async fn acquire_leadership(&self, cmd: AcquireLeadership) -> Result<bool, SchedulerError> {
        self.command_handler.acquire_leadership(cmd).await
    }
    pub async fn release_leadership(&self, cmd: ReleaseLeadership) -> Result<(), SchedulerError> {
        self.command_handler.release_leadership(cmd).await
    }
    pub async fn get_job(&self, job_id: Uuid) -> Result<ScheduledJob, SchedulerError> {
        self.query_handler.get_job(job_id).await
    }
    pub async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.query_handler.list_jobs().await
    }
    pub async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.query_handler.list_jobs_by_service(service).await
    }
    pub async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.query_handler.list_active_jobs().await
    }
    pub async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        self.query_handler.list_executions(job_id).await
    }
    pub async fn list_default_jobs(&self) -> Vec<(String, String, JobType, JobSchedule)> {
        self.query_handler.list_default_jobs().await
    }
}

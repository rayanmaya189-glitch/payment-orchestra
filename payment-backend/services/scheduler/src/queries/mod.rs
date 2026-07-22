//! Scheduler query handlers

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_job(&self, job_id: Uuid) -> Result<ScheduledJob, SchedulerError>;
    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError>;
    async fn list_default_jobs(&self) -> Vec<(String, String, JobType, JobSchedule)>;
}

pub struct SchedulerQueryHandler<R: SchedulerRepository> {
    repo: R,
}

impl<R: SchedulerRepository> SchedulerQueryHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: SchedulerRepository + Send + Sync> QueryHandler for SchedulerQueryHandler<R> {
    async fn get_job(&self, job_id: Uuid) -> Result<ScheduledJob, SchedulerError> {
        self.repo.load_job(job_id).await?.ok_or(SchedulerError::JobNotFound(job_id))
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.repo.list_jobs().await
    }

    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.repo.list_jobs_by_service(service).await
    }

    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.repo.list_active_jobs().await
    }

    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        self.repo.list_executions(job_id).await
    }

    async fn list_default_jobs(&self) -> Vec<(String, String, JobType, JobSchedule)> {
        default_jobs()
    }
}

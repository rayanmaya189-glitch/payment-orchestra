//! Scheduler commands

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn register_job(&self, cmd: RegisterJob) -> Result<ScheduledJob, SchedulerError>;
    async fn execute_job(&self, cmd: ExecuteJob) -> Result<JobExecution, SchedulerError>;
    async fn pause_job(&self, cmd: PauseJob) -> Result<ScheduledJob, SchedulerError>;
    async fn resume_job(&self, cmd: ResumeJob) -> Result<ScheduledJob, SchedulerError>;
    async fn acquire_leadership(&self, cmd: AcquireLeadership) -> Result<bool, SchedulerError>;
    async fn release_leadership(&self, cmd: ReleaseLeadership) -> Result<(), SchedulerError>;
}

pub struct SchedulerCommandHandler<R: SchedulerRepository> {
    repo: R,
}

impl<R: SchedulerRepository> SchedulerCommandHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: SchedulerRepository + Send + Sync> CommandHandler for SchedulerCommandHandler<R> {
    async fn register_job(&self, cmd: RegisterJob) -> Result<ScheduledJob, SchedulerError> {
        if let Some(_existing) = self.repo.load_job_by_key(&cmd.job_key).await? {
            return Err(SchedulerError::JobKeyConflict(cmd.job_key));
        }

        let now = Utc::now();
        let job = ScheduledJob {
            job_id: Uuid::now_v7(),
            job_key: cmd.job_key,
            service_name: cmd.service_name,
            description: cmd.description,
            schedule: cmd.schedule,
            job_type: cmd.job_type,
            status: JobStatus::Active,
            last_run_at: None,
            last_result: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        };
        self.repo.save_job(&job).await?;
        Ok(job)
    }

    async fn execute_job(&self, cmd: ExecuteJob) -> Result<JobExecution, SchedulerError> {
        let mut job = self.repo.load_job(cmd.job_id).await?
            .ok_or(SchedulerError::JobNotFound(cmd.job_id))?;

        if job.status != JobStatus::Active {
            return Err(SchedulerError::JobDisabled(job.job_key));
        }

        let now = Utc::now();
        let execution = JobExecution {
            execution_id: Uuid::now_v7(),
            job_id: cmd.job_id,
            job_key: job.job_key.clone(),
            status: if cmd.success { ExecutionStatus::Completed } else { ExecutionStatus::Failed },
            started_at: now,
            completed_at: Some(now),
            duration_ms: Some(cmd.duration_ms),
            result: Some(JobRunResult {
                success: cmd.success,
                started_at: job.last_run_at.unwrap_or(now),
                completed_at: now,
                duration_ms: cmd.duration_ms,
                error_message: cmd.error_message,
            }),
        };

        job.last_run_at = Some(now);
        job.last_result = execution.result.clone();
        job.updated_at = now;
        self.repo.save_job(&job).await?;
        self.repo.save_execution(&execution).await?;
        Ok(execution)
    }

    async fn pause_job(&self, cmd: PauseJob) -> Result<ScheduledJob, SchedulerError> {
        let mut job = self.repo.load_job(cmd.job_id).await?
            .ok_or(SchedulerError::JobNotFound(cmd.job_id))?;
        job.status = JobStatus::Paused;
        job.updated_at = Utc::now();
        self.repo.save_job(&job).await?;
        Ok(job)
    }

    async fn resume_job(&self, cmd: ResumeJob) -> Result<ScheduledJob, SchedulerError> {
        let mut job = self.repo.load_job(cmd.job_id).await?
            .ok_or(SchedulerError::JobNotFound(cmd.job_id))?;
        job.status = JobStatus::Active;
        job.updated_at = Utc::now();
        self.repo.save_job(&job).await?;
        Ok(job)
    }

    async fn acquire_leadership(&self, cmd: AcquireLeadership) -> Result<bool, SchedulerError> {
        self.repo.acquire_lease(&cmd.job_key, &cmd.leader_id, cmd.ttl_secs).await
    }

    async fn release_leadership(&self, cmd: ReleaseLeadership) -> Result<(), SchedulerError> {
        self.repo.release_lease(&cmd.job_key, &cmd.leader_id).await
    }
}

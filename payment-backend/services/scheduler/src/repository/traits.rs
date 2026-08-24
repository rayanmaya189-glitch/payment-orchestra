//! Scheduler repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait SchedulerRepository: Send + Sync {
    async fn load_job(&self, job_id: Uuid) -> Result<Option<ScheduledJob>, SchedulerError>;
    async fn load_job_by_key(&self, job_key: &str) -> Result<Option<ScheduledJob>, SchedulerError>;
    async fn save_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError>;
    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError>;
    async fn save_execution(&self, execution: &JobExecution) -> Result<(), SchedulerError>;
    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError>;
    async fn acquire_lease(&self, job_key: &str, leader_id: &str, ttl_secs: u64) -> Result<bool, SchedulerError>;
    async fn release_lease(&self, job_key: &str, leader_id: &str) -> Result<(), SchedulerError>;
}

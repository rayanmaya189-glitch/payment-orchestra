//! In-memory Scheduler repository.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::SchedulerRepository;

#[derive(Clone)]
pub struct InMemorySchedulerRepository {
    pub(super) jobs: Arc<RwLock<HashMap<Uuid, ScheduledJob>>>,
    pub(super) executions: Arc<RwLock<Vec<JobExecution>>>,
    pub(super) leases: Arc<RwLock<HashMap<String, LeaderLease>>>,
}

impl Default for InMemorySchedulerRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySchedulerRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SchedulerRepository for InMemorySchedulerRepository {
    async fn load_job(&self, job_id: Uuid) -> Result<Option<ScheduledJob>, SchedulerError> {
        Ok(self.jobs.read().await.get(&job_id).cloned())
    }

    async fn load_job_by_key(&self, job_key: &str) -> Result<Option<ScheduledJob>, SchedulerError> {
        let map = self.jobs.read().await;
        Ok(map.values().find(|j| j.job_key == job_key).cloned())
    }

    async fn save_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        self.jobs.write().await.insert(job.job_id, job.clone());
        Ok(())
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        Ok(self.jobs.read().await.values().cloned().collect())
    }

    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let map = self.jobs.read().await;
        Ok(map.values().filter(|j| j.service_name == service || j.service_name == "all").cloned().collect())
    }

    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let map = self.jobs.read().await;
        Ok(map.values().filter(|j| j.status == JobStatus::Active).cloned().collect())
    }

    async fn save_execution(&self, execution: &JobExecution) -> Result<(), SchedulerError> {
        self.executions.write().await.push(execution.clone());
        Ok(())
    }

    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        let execs = self.executions.read().await;
        Ok(execs.iter().filter(|e| e.job_id == job_id).cloned().collect())
    }

    async fn acquire_lease(&self, job_key: &str, leader_id: &str, ttl_secs: u64) -> Result<bool, SchedulerError> {
        let mut leases = self.leases.write().await;
        let now = Utc::now();
        if let Some(lease) = leases.get(job_key) {
            if lease.leader_id != leader_id && lease.expires_at > now { return Ok(false); }
        }
        leases.insert(job_key.into(), LeaderLease {
            leader_id: leader_id.into(), job_key: job_key.into(),
            acquired_at: now, expires_at: now + chrono::Duration::seconds(ttl_secs as i64),
        });
        Ok(true)
    }

    async fn release_lease(&self, job_key: &str, leader_id: &str) -> Result<(), SchedulerError> {
        let mut leases = self.leases.write().await;
        if let Some(lease) = leases.get(job_key) {
            if lease.leader_id == leader_id { leases.remove(job_key); }
        }
        Ok(())
    }
}

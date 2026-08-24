//! Scheduler Service pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SchedulerPipeline {
    pub api: SchedulerApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemorySchedulerRepository>>,
}

use platform_messaging::event_bus::{EventBus, NoopEventBus};

impl Default for SchedulerPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemorySchedulerRepository::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let ch: Box<dyn CommandHandler> = Box::new(SchedulerCommandHandler::new(adapter.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(SchedulerQueryHandler::new(adapter));
        let api = SchedulerApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self { api, event_bus: eb, repo }
    }

    /// Override the default no-op event bus with a real NATS-backed bus.
    pub fn with_event_bus(mut self, event_bus: Box<dyn EventBus>) -> Self {
        self.event_bus = event_bus;
        self
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemorySchedulerRepository>>);

#[async_trait::async_trait]
impl SchedulerRepository for ArcRepoAdapter {
    async fn load_job(&self, id: uuid::Uuid) -> Result<Option<ScheduledJob>, SchedulerError> {
        self.0.read().await.load_job(id).await
    }
    async fn load_job_by_key(&self, key: &str) -> Result<Option<ScheduledJob>, SchedulerError> {
        self.0.read().await.load_job_by_key(key).await
    }
    async fn save_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        self.0.write().await.save_job(job).await
    }
    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.0.read().await.list_jobs().await
    }
    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.0.read().await.list_jobs_by_service(service).await
    }
    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.0.read().await.list_active_jobs().await
    }
    async fn save_execution(&self, exec: &JobExecution) -> Result<(), SchedulerError> {
        self.0.write().await.save_execution(exec).await
    }
    async fn list_executions(&self, id: uuid::Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        self.0.read().await.list_executions(id).await
    }
    async fn acquire_lease(&self, key: &str, leader_id: &str, ttl: u64) -> Result<bool, SchedulerError> {
        self.0.write().await.acquire_lease(key, leader_id, ttl).await
    }
    async fn release_lease(&self, key: &str, leader_id: &str) -> Result<(), SchedulerError> {
        self.0.write().await.release_lease(key, leader_id).await
    }
}

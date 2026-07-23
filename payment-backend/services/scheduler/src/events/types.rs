//! Scheduler events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulerEvent {
    JobRegistered(JobRegisteredPayload),
    JobExecuted(JobExecutedPayload),
    JobFailed(JobFailedPayload),
    LeaderElected(LeaderElectedPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRegisteredPayload {
    pub job_id: Uuid,
    pub job_key: String,
    pub service_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecutedPayload {
    pub job_id: Uuid,
    pub execution_id: Uuid,
    pub duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFailedPayload {
    pub job_id: Uuid,
    pub execution_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectedPayload {
    pub job_key: String,
    pub leader_id: String,
    pub acquired_at: DateTime<Utc>,
}

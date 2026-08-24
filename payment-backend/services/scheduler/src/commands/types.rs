//! Command types for Scheduler

use uuid::Uuid;
use crate::domain::*;

pub struct RegisterJob {
    pub job_key: String,
    pub service_name: String,
    pub description: String,
    pub schedule: JobSchedule,
    pub job_type: JobType,
}

pub struct ExecuteJob {
    pub job_id: Uuid,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

pub struct PauseJob {
    pub job_id: Uuid,
}

pub struct ResumeJob {
    pub job_id: Uuid,
}

pub struct AcquireLeadership {
    pub job_key: String,
    pub leader_id: String,
    pub ttl_secs: u64,
}

pub struct ReleaseLeadership {
    pub job_key: String,
    pub leader_id: String,
}

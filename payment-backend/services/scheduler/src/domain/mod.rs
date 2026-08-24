//! Scheduler domain model
//!
//! In-process cron-style scheduling with leader election (LEADER-001).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ScheduledJob — a job definition with its schedule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub job_id: Uuid,
    pub job_key: String,
    pub service_name: String,
    pub description: String,
    pub schedule: JobSchedule,
    pub job_type: JobType,
    pub status: JobStatus,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<JobRunResult>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobSchedule {
    /// Run at a fixed interval (seconds).
    Every { interval_seconds: u64 },
    /// Run at a specific cron expression.
    Cron { expression: String },
    /// Run once at a specific time.
    OnceAt { run_at: DateTime<Utc> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    SubscriptionRenewal,
    DunningRetry,
    SettlementPolling,
    InvoiceOverdue,
    AiReEmbedding,
    ExceptionAging,
    AuthExpirySweep,
    StuckAuthorizing,
    DataRetention,
    OutboxRelayHealth,
    AuditHashVerify,
    FeeVarianceReport,
    ChargebackDeadline,
    Custom(String),
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubscriptionRenewal => write!(f, "subscription_renewal"),
            Self::DunningRetry => write!(f, "dunning_retry"),
            Self::SettlementPolling => write!(f, "settlement_polling"),
            Self::InvoiceOverdue => write!(f, "invoice_overdue"),
            Self::AiReEmbedding => write!(f, "ai_re_embedding"),
            Self::ExceptionAging => write!(f, "exception_aging"),
            Self::AuthExpirySweep => write!(f, "auth_expiry_sweep"),
            Self::StuckAuthorizing => write!(f, "stuck_authorizing"),
            Self::DataRetention => write!(f, "data_retention"),
            Self::OutboxRelayHealth => write!(f, "outbox_relay_health"),
            Self::AuditHashVerify => write!(f, "audit_hash_verify"),
            Self::FeeVarianceReport => write!(f, "fee_variance_report"),
            Self::ChargebackDeadline => write!(f, "chargeback_deadline"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Active,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRunResult {
    pub success: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

// ---------------------------------------------------------------------------
// JobExecution — a single run of a job
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecution {
    pub execution_id: Uuid,
    pub job_id: Uuid,
    pub job_key: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub result: Option<JobRunResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
}

// ---------------------------------------------------------------------------
// LeaderElection — simple in-memory leader election
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderLease {
    pub leader_id: String,
    pub job_key: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum SchedulerError {
    #[error("Job not found: {0}")]
    JobNotFound(Uuid),
    #[error("Job key already exists: {0}")]
    JobKeyConflict(String),
    #[error("Invalid schedule configuration: {0}")]
    InvalidSchedule(String),
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Job is disabled: {0}")]
    JobDisabled(String),
    #[error("Not the leader for job: {0}")]
    NotLeader(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

// ---------------------------------------------------------------------------
// Default job registry
// ---------------------------------------------------------------------------

pub fn default_jobs() -> Vec<(String, String, JobType, JobSchedule)> {
    vec![
        ("auth_expiry_sweep".into(), "orchestration-service".into(), JobType::AuthExpirySweep, JobSchedule::Every { interval_seconds: 300 }),
        ("stuck_authorizing".into(), "orchestration-service".into(), JobType::StuckAuthorizing, JobSchedule::Every { interval_seconds: 60 }),
        ("subscription_renewal".into(), "subscription-service".into(), JobType::SubscriptionRenewal, JobSchedule::Every { interval_seconds: 3600 }),
        ("dunning_retry".into(), "subscription-service".into(), JobType::DunningRetry, JobSchedule::Every { interval_seconds: 1800 }),
        ("settlement_polling".into(), "reconciliation-service".into(), JobType::SettlementPolling, JobSchedule::Every { interval_seconds: 900 }),
        ("invoice_overdue".into(), "invoice-service".into(), JobType::InvoiceOverdue, JobSchedule::Every { interval_seconds: 86400 }),
        ("ai_re_embedding".into(), "ai-assistant-service".into(), JobType::AiReEmbedding, JobSchedule::Every { interval_seconds: 3600 }),
        ("data_retention".into(), "all".into(), JobType::DataRetention, JobSchedule::Every { interval_seconds: 86400 }),
        ("outbox_relay_health".into(), "all".into(), JobType::OutboxRelayHealth, JobSchedule::Every { interval_seconds: 30 }),
        ("chargeback_deadline".into(), "dispute-service".into(), JobType::ChargebackDeadline, JobSchedule::Every { interval_seconds: 86400 }),
    ]
}

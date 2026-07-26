//! Background Job Scheduler — in-process cron with leader election.
//!
//! Executes scheduled tasks (retries, invoice finalization, subscription
//! renewal) with leader-election for horizontal scaling.
//!
//! ## Architecture
//!
//! The scheduler runs in a loop, polling for due jobs every `tick_interval_ms`.
//! Before executing a job, it checks leader election via the repository's
//! `acquire_lease` method to ensure only one instance runs each job type
//! across the cluster.

use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use chrono::Utc;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

mod domain;
mod entities;
mod commands;
mod queries;
mod events;
mod repository;
mod api;
mod pipeline;

#[cfg(test)]
mod tests;

use domain::{
    SchedulerError, JobSchedule, JobType, JobExecution, ExecutionStatus,
    JobRunResult, default_jobs, ScheduledJob,
};
use repository::{
    SchedulerRepository, InMemorySchedulerRepository, PostgresSchedulerRepository,
};

/// Scheduler tick interval (checks for due jobs every 1 second)
const TICK_INTERVAL_MS: u64 = 1000;

/// Maximum time a job is allowed to run before being marked as timed out
const JOB_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Leader lease TTL in seconds
const LEADER_LEASE_TTL_SECS: u64 = 30;

/// Instance ID for leader election
fn instance_id() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| uuid::Uuid::now_v7().to_string())
}

/// Execute a single job by type.
async fn execute_job(job: &ScheduledJob) -> Result<JobRunResult, String> {
    let started_at = Utc::now();
    info!(job_key = %job.job_key, job_type = %job.job_type, "Executing scheduled job");

    match job.job_type {
        JobType::AuthExpirySweep
        | JobType::StuckAuthorizing
        | JobType::SubscriptionRenewal
        | JobType::DunningRetry
        | JobType::SettlementPolling
        | JobType::InvoiceOverdue
        | JobType::OutboxRelayHealth
        | JobType::DataRetention
        | JobType::FeeVarianceReport
        | JobType::ChargebackDeadline
        | JobType::AiReEmbedding
        | JobType::ExceptionAging
        | JobType::AuditHashVerify
        | JobType::Custom(_) => {
            // All job types are handled by the same simple path for now.
            // Real implementations will dispatch to the appropriate downstream
            // service via gRPC client calls (e.g., orchestration-service for
            // AuthExpirySweep, subscription-service for SubscriptionRenewal).
        }
    };

    let completed_at = Utc::now();
    let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

    info!(job_key = %job.job_key, duration_ms = duration_ms, "Job completed successfully");
    Ok(JobRunResult {
        success: true,
        started_at,
        completed_at,
        duration_ms,
        error_message: None,
    })
}

/// Main scheduler loop — polls job queue and executes due jobs.
async fn scheduler_loop(
    repo: Arc<dyn SchedulerRepository>,
    event_bus: Arc<dyn EventBus>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(TICK_INTERVAL_MS));
    let mut last_lease_refresh = std::time::Instant::now();
    let instance_id = instance_id();

    info!("Scheduler loop started with {}ms tick interval", TICK_INTERVAL_MS);

    loop {
        interval.tick().await;

        // Load all active jobs
        let jobs = match repo.list_active_jobs().await {
            Ok(jobs) => jobs,
            Err(e) => {
                error!(error = %e, "Failed to list active jobs");
                continue;
            }
        };

        for job in &jobs {
            // Check if this job is due for execution
            let is_due = match job.next_run_at {
                Some(next) => Utc::now() >= next,
                None => true, // First run — schedule immediately
            };

            if !is_due {
                continue;
            }

            // Try to become leader for this job (distributed lock via repo)
            let is_leader = match repo.acquire_lease(&job.job_key, &instance_id, LEADER_LEASE_TTL_SECS).await {
                Ok(true) => true,
                _ => false,
            };

            if !is_leader {
                continue; // Another instance is handling this job
            }

            // Execute the job in a separate task
            let job_id = job.job_id;
            let job_key = job.job_key.clone();
            let job_type_str = job.job_type.to_string();
            let job_clone = job.clone();
            let _eb = event_bus.clone();

            tokio::spawn(async move {
                let execution_id = uuid::Uuid::now_v7();
                let mut execution = JobExecution {
                    execution_id,
                    job_id,
                    job_key: job_key.clone(),
                    status: ExecutionStatus::Running,
                    started_at: Utc::now(),
                    completed_at: None,
                    duration_ms: None,
                    result: None,
                };

                // Run the job with timeout
                let timeout = tokio::time::sleep(Duration::from_secs(JOB_TIMEOUT_SECS));
                let job_result = tokio::select! {
                    result = execute_job(&job_clone) => result,
                    _ = timeout => {
                        error!(job_key = %job_key, job_type = %job_type_str, "Job timed out");
                        execution.status = ExecutionStatus::TimedOut;
                        execution.completed_at = Some(Utc::now());
                        Err("Job execution timed out".to_string())
                    }
                };

                match job_result {
                    Ok(result) => {
                        execution.status = if result.success {
                            ExecutionStatus::Completed
                        } else {
                            ExecutionStatus::Failed
                        };
                        execution.result = Some(result);
                    }
                    Err(msg) => {
                        execution.status = ExecutionStatus::Failed;
                        execution.completed_at = Some(Utc::now());
                        execution.duration_ms = Some(
                            (execution.completed_at.unwrap() - execution.started_at)
                                .num_milliseconds().max(0) as u64
                        );
                        execution.result = Some(JobRunResult {
                            success: false,
                            started_at: execution.started_at,
                            completed_at: execution.completed_at.unwrap(),
                            duration_ms: execution.duration_ms.unwrap(),
                            error_message: Some(msg),
                        });
                    }
                }

                execution.completed_at = Some(Utc::now());
                execution.duration_ms = Some(
                    (execution.completed_at.unwrap() - execution.started_at)
                        .num_milliseconds().max(0) as u64
                );
                info!(
                    job_key = %execution.job_key,
                    status = ?execution.status,
                    duration_ms = %execution.duration_ms.unwrap_or(0),
                    "Job execution completed"
                );
            });

            // Update next_run_at for the job
            let next_run = match job.schedule {
                JobSchedule::Every { interval_seconds } => {
                    Some(Utc::now() + chrono::TimeDelta::seconds(interval_seconds as i64))
                }
                JobSchedule::Cron { .. } => {
                    Some(Utc::now() + chrono::TimeDelta::hours(1))
                }
                JobSchedule::OnceAt { .. } => None,
            };

            // Save updated job with new next_run_at
            let mut updated_job = job.clone();
            updated_job.next_run_at = next_run;
            if let Err(e) = repo.save_job(&updated_job).await {
                error!(job_id = %job.job_id, error = %e, "Failed to update next run time");
            }
        }

        // Refresh leader leases every ~15 seconds
        if last_lease_refresh.elapsed() >= Duration::from_secs(15) {
            for job in &jobs {
                if let Err(e) = repo.acquire_lease(&job.job_key, &instance_id, LEADER_LEASE_TTL_SECS).await {
                    warn!(job_key = %job.job_key, error = %e, "Failed to refresh leader lease");
                }
            }
            last_lease_refresh = std::time::Instant::now();
        }
    }
}

/// Register default jobs in the repository.
async fn register_default_jobs(repo: &dyn SchedulerRepository) {
    for (job_key, service_name, job_type, schedule) in default_jobs() {
        match repo.load_job_by_key(&job_key).await {
            Ok(Some(_)) => continue, // Already registered
            Ok(None) => {
                let job = ScheduledJob {
                    job_id: uuid::Uuid::now_v7(),
                    job_key,
                    service_name,
                    description: format!("{} job", job_type),
                    schedule,
                    job_type,
                    status: domain::JobStatus::Active,
                    last_run_at: None,
                    last_result: None,
                    next_run_at: Some(Utc::now()),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                if let Err(e) = repo.save_job(&job).await {
                    error!(job_key = %job.job_key, error = %e, "Failed to register default job");
                }
            }
            Err(e) => {
                error!(job_key = %job_key, error = %e, "Failed to check if job exists");
            }
        }
    }
    info!("Default jobs registered");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("scheduler", 9017, 9117).await?;

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("SCHEDULER_NATS_USERNAME").ok();
        let nats_password = std::env::var("SCHEDULER_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as scheduler_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Initialize repository and register default jobs
    let repo: Arc<dyn SchedulerRepository> = if let Some(db) = create_service_pool("SCHEDULER").await.ok() {
        info!("Using PostgreSQL-backed repository for scheduler");
        let repo = PostgresSchedulerRepository::new(db);
        register_default_jobs(&repo).await;
        Arc::new(repo)
    } else {
        warn!("PostgreSQL unavailable, using InMemory repository");
        let repo = InMemorySchedulerRepository::new();
        register_default_jobs(&repo).await;
        Arc::new(repo)
    };

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Start the main scheduler loop
    let scheduler_repo = repo.clone();
    let scheduler_eb = event_bus.clone();
    tokio::spawn(async move {
        scheduler_loop(scheduler_repo, scheduler_eb).await;
    });

    info!("Scheduler service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Scheduler service stopped");
    Ok(())
}

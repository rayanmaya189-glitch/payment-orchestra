//! Background Job Scheduler — in-process cron with leader election.
//!
//! Executes scheduled tasks (retries, invoice finalization, subscription
//! renewal) with leader-election for horizontal scaling.
//!
//! ## Architecture
//!
//! The scheduler runs a background loop that polls for due jobs every
//! `TICK_INTERVAL_MS` and executes them with leader election. A gRPC
//! management service (`SchedulerService`) provides operational RPCs
//! for inspecting and managing jobs.

// Scaffold modules contain intentionally unused code for future implementation.
#![allow(
    dead_code,
    clippy::result_large_err,
    clippy::match_like_matches_macro
)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use chrono::Utc;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

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
    JobSchedule, JobType, JobExecution, ExecutionStatus,
    JobRunResult, default_jobs, ScheduledJob,
};
use repository::{
    SchedulerRepository, InMemorySchedulerRepository, PostgresSchedulerRepository,
};
use api::grpc::SchedulerGrpcService;
use platform_proto::health::health_server::HealthServer;
use platform_proto::scheduler::scheduler_service_server::SchedulerServiceServer;
use platform_health::grpc::HealthService;

// ─── Constants ──────────────────────────────────────────────────────────────

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

// ─── Job Execution ──────────────────────────────────────────────────────────

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

// ─── Scheduler Loop ─────────────────────────────────────────────────────────

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

        let jobs = match repo.list_active_jobs().await {
            Ok(jobs) => jobs,
            Err(e) => {
                error!(error = %e, "Failed to list active jobs");
                continue;
            }
        };

        for job in &jobs {
            let is_due = match job.next_run_at {
                Some(next) => Utc::now() >= next,
                None => true,
            };

            if !is_due {
                continue;
            }

            let is_leader = match repo.acquire_lease(&job.job_key, &instance_id, LEADER_LEASE_TTL_SECS).await {
                Ok(true) => true,
                _ => false,
            };

            if !is_leader {
                continue;
            }

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
                        execution.status = if result.success { ExecutionStatus::Completed } else { ExecutionStatus::Failed };
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

            let next_run = match job.schedule {
                JobSchedule::Every { interval_seconds } => {
                    Some(Utc::now() + chrono::TimeDelta::seconds(interval_seconds as i64))
                }
                JobSchedule::Cron { .. } => Some(Utc::now() + chrono::TimeDelta::hours(1)),
                JobSchedule::OnceAt { .. } => None,
            };

            let mut updated_job = job.clone();
            updated_job.next_run_at = next_run;
            if let Err(e) = repo.save_job(&updated_job).await {
                error!(job_id = %job.job_id, error = %e, "Failed to update next run time");
            }
        }

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

// ─── Default Job Registration ───────────────────────────────────────────────

/// Register default jobs in the repository.
async fn register_default_jobs(repo: &dyn SchedulerRepository) {
    for (job_key, service_name, job_type, schedule) in default_jobs() {
        match repo.load_job_by_key(&job_key).await {
            Ok(Some(_)) => continue,
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

// ─── Main ───────────────────────────────────────────────────────────────────

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

    // Initialize repository, register default jobs, and build handlers
    use crate::commands::SchedulerCommandHandler;
    use crate::queries::SchedulerQueryHandler;

    let (api, loop_repo): (crate::api::SchedulerApi, Arc<dyn SchedulerRepository>) = {
        let (repo, ch, qh): (Arc<dyn SchedulerRepository>, _, _) =
            if let Ok(db) = create_service_pool("SCHEDULER").await {
                info!("Using PostgreSQL-backed repository for scheduler");
                let repo = PostgresSchedulerRepository::new(db);
                register_default_jobs(&repo).await;
                let ch: Box<dyn crate::commands::CommandHandler> =
                    Box::new(SchedulerCommandHandler::new(repo.clone()));
                let qh: Box<dyn crate::queries::QueryHandler> =
                    Box::new(SchedulerQueryHandler::new(repo.clone()));
                (Arc::new(repo) as Arc<dyn SchedulerRepository>, ch, qh)
            } else {
                warn!("PostgreSQL unavailable, using InMemory repository");
                let mem_repo = InMemorySchedulerRepository::new();
                register_default_jobs(&mem_repo).await;
                let rw_mem = Arc::new(tokio::sync::RwLock::new(mem_repo));
                let adapter = pipeline::ArcRepoAdapter(rw_mem);
                let ch: Box<dyn crate::commands::CommandHandler> =
                    Box::new(SchedulerCommandHandler::new(adapter.clone()));
                let qh: Box<dyn crate::queries::QueryHandler> =
                    Box::new(SchedulerQueryHandler::new(adapter.clone()));
                // ArcRepoAdapter implements SchedulerRepository, so wrap in Arc<dyn ...>
                (Arc::new(adapter) as Arc<dyn SchedulerRepository>, ch, qh)
            };
        (crate::api::SchedulerApi::new(ch, qh), repo)
    };
    let scheduler_service = SchedulerGrpcService::new(api);
    let health_service = HealthService::new("scheduler".to_string());

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Scheduler gRPC server listening on {grpc_addr}");

    // Spawn periodic uptime recording
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Start the scheduler loop as a background task
    let scheduler_repo = loop_repo.clone();
    let scheduler_eb = event_bus.clone();
    tokio::spawn(async move {
        scheduler_loop(scheduler_repo, scheduler_eb).await;
    });

    // Run the gRPC server with graceful shutdown
    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(MetricsLayer::new("scheduler"))
            .layer(GrcRateLimitLayer::in_memory("scheduler"))
            .add_service(SchedulerServiceServer::new(scheduler_service))
            .add_service(HealthServer::new(health_service))
            .serve_with_shutdown(grpc_addr, async {
                tokio::signal::ctrl_c().await.ok();
            }) => {
                if let Err(e) = result {
                    tracing::error!("gRPC server error: {}", e);
                }
            }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    platform_logging::telemetry::shutdown();
    info!("Scheduler service stopped");
    Ok(())
}

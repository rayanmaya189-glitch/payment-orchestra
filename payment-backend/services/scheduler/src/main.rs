//! Background Job Scheduler — in-process cron with leader election.
//!
//! Executes scheduled tasks (retries, invoice finalization, subscription
//! renewal) with leader-election for horizontal scaling.

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("scheduler", 9017, 9117).await?;

    info!("Scheduler service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Scheduler service stopped");
    Ok(())
}

//! Analytics Service
//! BC-15: Metrics, reporting, analytics queries

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("analytics-service", 9015, 9115).await?;

    info!("Analytics service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Analytics service stopped");
    Ok(())
}

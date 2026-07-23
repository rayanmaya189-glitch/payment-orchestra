//! Subscription Billing Service
//! SVC-08: Subscription lifecycle, dunning, event-sourced

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("subscription-service", 9008, 9108).await?;

    info!("Subscription service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Subscription service stopped");
    Ok(())
}

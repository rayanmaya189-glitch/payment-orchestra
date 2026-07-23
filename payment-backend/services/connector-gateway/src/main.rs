//! Connector Gateway
//! BC-04: Acquirer connector framework, circuit breaker, adapters

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("connector-gateway", 9004, 9104).await?;

    info!("Connector Gateway service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Connector Gateway service stopped");
    Ok(())
}

//! Connector Gateway
//! BC-04: Acquirer connector framework, circuit breaker, adapters

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("connector-gateway", 9004, 9104).await?;

    info!("Connector Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Connector Gateway service stopped");
    Ok(())
}

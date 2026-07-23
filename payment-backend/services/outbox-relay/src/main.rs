//! Outbox Relay
//! Background task: polls outbox table, publishes to NATS channels

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("outbox-relay", 9019, 9119).await?;

    info!("Outbox Relay service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Outbox Relay service stopped");
    Ok(())
}

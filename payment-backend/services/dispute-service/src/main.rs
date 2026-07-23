//! Dispute Service
//! SVC-10: Chargeback lifecycle, representment, CRUD + events

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("dispute-service", 9010, 9110).await?;

    info!("Dispute service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Dispute service stopped");
    Ok(())
}

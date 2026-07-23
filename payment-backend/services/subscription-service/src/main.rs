//! Subscription Billing Service
//! SVC-08: Subscription lifecycle, dunning, event-sourced

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("subscription-service", 9008, 9108).await?;

    info!("Subscription service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Subscription service stopped");
    Ok(())
}

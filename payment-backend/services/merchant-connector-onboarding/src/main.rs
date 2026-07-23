//! Merchant Connector Onboarding
//! BYOK flow: create link, test connection, activate

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("merchant-connector-onboarding", 9022, 9122).await?;

    info!("Merchant Connector Onboarding service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Merchant Connector Onboarding service stopped");
    Ok(())
}

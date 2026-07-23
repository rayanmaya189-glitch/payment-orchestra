//! Payment Link Service
//! BC-07: Hosted payment links, CRUD + events

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("payment-link-service", 9007, 9107).await?;

    info!("Payment Link service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Payment Link service stopped");
    Ok(())
}

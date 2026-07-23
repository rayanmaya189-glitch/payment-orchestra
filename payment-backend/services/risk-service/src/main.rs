//! Risk Service
//! SVC-11: Fraud scoring, rules engine, synchronous scoring

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("risk-service", 9011, 9111).await?;

    info!("Risk service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Risk service stopped");
    Ok(())
}

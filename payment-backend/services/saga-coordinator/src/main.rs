//! Saga Coordinator — BC-17 Durable state machine runtime.
//!
//! Coordinates multi-step cross-aggregate workflows with
//! compensation-capable sagas.

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("saga-coordinator", 9016, 9116).await?;

    info!("Saga Coordinator service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Saga Coordinator service stopped");
    Ok(())
}

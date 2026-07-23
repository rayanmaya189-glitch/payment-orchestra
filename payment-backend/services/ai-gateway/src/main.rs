//! AI Gateway
//! Guardrail layer: prompt screening, circuit breaker, quotas

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("ai-gateway", 9021, 9121).await?;

    info!("AI Gateway service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("AI Gateway service stopped");
    Ok(())
}

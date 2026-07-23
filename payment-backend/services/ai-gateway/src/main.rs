//! AI Gateway
//! Guardrail layer: prompt screening, circuit breaker, quotas

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("ai-gateway", 9021, 9121).await?;

    info!("AI Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("AI Gateway service stopped");
    Ok(())
}

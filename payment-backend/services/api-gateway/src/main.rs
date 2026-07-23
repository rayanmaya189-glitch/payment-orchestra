//! API Gateway
//! External ingress: REST paths + protobuf bodies, auth, rate limiting

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("api-gateway", 9020, 9120).await?;

    info!("API Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("API Gateway service stopped");
    Ok(())
}

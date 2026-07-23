//! API Gateway
//! External ingress: REST paths + protobuf bodies, auth, rate limiting

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("api-gateway", 9020, 9120).await?;

    // Initialize gRPC clients for downstream services
    // (used when routing external requests to internal services)
    let iam_addr = format!("http://127.0.0.1:{}", std::env::var("iam_service_grpc_port").unwrap_or_else(|_| "9002".into()));
    let _iam_client = platform_clients::iam::IamClient::connect(&iam_addr).await?;
    tracing::info!(addr = %iam_addr, "IAM gRPC client connected");

    let orchestration_addr = format!("http://127.0.0.1:{}", std::env::var("orchestration_service_grpc_port").unwrap_or_else(|_| "9005".into()));
    let _orchestration_client = platform_clients::orchestration::OrchestrationClient::connect(&orchestration_addr).await?;
    tracing::info!(addr = %orchestration_addr, "Orchestration gRPC client connected");

    info!("API Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("API Gateway service stopped");
    Ok(())
}

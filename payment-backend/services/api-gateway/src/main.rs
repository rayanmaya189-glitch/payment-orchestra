//! API Gateway — External ingress for all REST+protobuf traffic.
//! SVC-17: TLS termination, auth, rate limiting, CORS, request routing.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("api-gateway starting...");
    // TODO: Initialize config, middleware pipeline, gRPC server
    Ok(())
}

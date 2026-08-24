//! Helper to start a gRPC health server for scaffold services.
//!
//! Every service that lacks a real gRPC implementation uses this to
//! bind its port and serve the Health checking protocol, making it
//! discoverable via etcd service discovery.

use platform_registry::bootstrap::ServerRunner;
use tonic::transport::Server;

use crate::grpc::HealthService;
use platform_proto::health::health_server::HealthServer;

/// Start a gRPC server that serves only the Health check protocol.
///
/// The server runs until Ctrl+C is received, then returns.
/// Caller should deregister from etcd after this returns.
pub async fn serve_health(runner: &ServerRunner) -> Result<(), Box<dyn std::error::Error>> {
    let health_service = HealthService::new(runner.service_name.clone());
    let grpc_addr = runner.grpc_addr;

    tracing::info!(
        service = %runner.service_name,
        addr = %grpc_addr,
        "Starting health-only gRPC server"
    );

    let server = Server::builder()
        .add_service(HealthServer::new(health_service))
        .serve(grpc_addr);

    tokio::select! {
        result = server => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!(service = %runner.service_name, "Shutdown signal received");
        }
    }

    Ok(())
}

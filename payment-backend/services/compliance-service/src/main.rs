//! Compliance Service — BC-03 Merchant Compliance
//!
//! Handles KYB (Know Your Business) case management,
//! AML transaction monitoring, and SAR report generation.

use std::net::SocketAddr;
use tracing::info;

mod domain;
mod commands;
mod queries;
mod events;
mod repository;
mod api;
mod pipeline;

#[cfg(test)]
mod tests;

use commands::ComplianceCommandHandler;
use queries::ComplianceQueries;
use repository::InMemoryComplianceRepository;
use api::grpc::ComplianceGrpcService;
use platform_proto::compliance::compliance_service_server::ComplianceServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("compliance-service", 9003, 9103).await?;

    let repository = InMemoryComplianceRepository::new();
    let command_handler = ComplianceCommandHandler::new(repository.clone());
    let queries = ComplianceQueries::new(repository);
    let compliance_service = ComplianceGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Compliance service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(ComplianceServiceServer::new(compliance_service))
            .serve_with_shutdown(addr, async {
                tokio::signal::ctrl_c().await.ok();
            }) => {
                if let Err(e) = result {
                    tracing::error!("gRPC server error: {}", e);
                }
            }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    info!("Compliance service stopped");
    Ok(())
}

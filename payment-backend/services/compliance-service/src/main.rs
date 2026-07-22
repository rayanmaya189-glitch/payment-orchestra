//! Compliance Service — BC-03 Merchant Compliance
//!
//! Handles KYB (Know Your Business) case management,
//! AML transaction monitoring, and SAR report generation.

use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server;
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
    platform_logging::telemetry::init();

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    info!(
        service = %config.service_name,
        listen_addr = %config.listen_addr,
        "Compliance service starting"
    );

    let repository = InMemoryComplianceRepository::new();
    let command_handler = ComplianceCommandHandler::new(repository.clone());
    let queries = ComplianceQueries::new(repository);

    let compliance_service = ComplianceGrpcService::new(command_handler, queries);

    let addr: SocketAddr = config.listen_addr.parse()
        .unwrap_or_else(|_| "0.0.0.0:9003".parse().unwrap());

    info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(ComplianceServiceServer::new(compliance_service))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c().await.ok();
            info!("Shutdown signal received");
        })
        .await?;

    info!("Compliance service stopped");
    Ok(())
}

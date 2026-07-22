//! Identity & Access Management Service — BC-02
//!
//! Handles authentication (Argon2id + JWT), ABAC authorization,
//! API key lifecycle, MFA enrollment, and Maker/Checker flows.

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

use commands::IamCommandHandler;
use queries::IamQueries;
use repository::InMemoryIamRepository;
use api::grpc::IamGrpcService;
use platform_proto::iam::iam_service_server::IamServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    info!(
        service = %config.service_name,
        listen_addr = %config.listen_addr,
        "IAM service starting"
    );

    let repository = InMemoryIamRepository::new();
    let command_handler = IamCommandHandler::new(
        repository.clone(),
        config.jwt_secret.clone(),
    );
    let queries = IamQueries::new(repository);

    let iam_service = IamGrpcService::new(command_handler, queries);

    let addr: SocketAddr = config.listen_addr.parse()
        .unwrap_or_else(|_| "0.0.0.0:9002".parse().unwrap());

    info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(IamServiceServer::new(iam_service))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c().await.ok();
            info!("Shutdown signal received");
        })
        .await?;

    info!("IAM service stopped");
    Ok(())
}

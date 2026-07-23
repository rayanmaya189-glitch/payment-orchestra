//! Identity & Access Management Service — BC-02
//!
//! Handles authentication (Argon2id + JWT), ABAC authorization,
//! API key lifecycle, MFA enrollment, and Maker/Checker flows.

use std::net::SocketAddr;
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
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("iam-service", 9002, 9102).await?;

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    let repository = InMemoryIamRepository::new();
    let command_handler = IamCommandHandler::new(
        repository.clone(),
        config.jwt_secret,
    );
    let queries = IamQueries::new(repository);
    let iam_service = IamGrpcService::new(command_handler, queries);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("IAM service gRPC server listening on {grpc_addr}");

    let server = Server::builder()
        .add_service(IamServiceServer::new(iam_service))
        .serve(grpc_addr);

    tokio::select! {
        result = server => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    info!("IAM service stopped");
    Ok(())
}

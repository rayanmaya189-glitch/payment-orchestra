//! Operator Service — BC-01 Operator Management
//! 
//! Handles operator (merchant) registration, email verification,
//! and lifecycle management. This is the first service to implement
//! as all other services depend on operator identity.

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

use commands::OperatorCommandHandler;
use queries::OperatorQueries;
use repository::InMemoryOperatorRepository;
use api::grpc::OperatorGrpcService;
use platform_proto::operator::operator_service_server::OperatorServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    platform_logging::telemetry::init();
    
    // Load configuration
    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();
    
    info!(
        service = %config.service_name,
        listen_addr = %config.listen_addr,
        log_level = %config.log_level,
        "Operator service starting"
    );

    // Initialize repository (in-memory for now, SeaORM in production)
    let repository = InMemoryOperatorRepository::new();
    
    // Create command handler and queries
    let command_handler = OperatorCommandHandler::new(repository.clone());
    let queries = OperatorQueries::new(repository);
    
    // Create gRPC service
    let operator_service = OperatorGrpcService::new(command_handler, queries);
    
    // Parse listen address
    let addr: SocketAddr = config.listen_addr.parse()
        .unwrap_or_else(|_| "0.0.0.0:9001".parse().unwrap());
    
    info!("gRPC server listening on {}", addr);

    // Start gRPC server with graceful shutdown
    Server::builder()
        .add_service(OperatorServiceServer::new(operator_service))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c().await.ok();
            info!("Shutdown signal received, starting graceful shutdown");
        })
        .await?;

    info!("Operator service stopped");
    Ok(())
}

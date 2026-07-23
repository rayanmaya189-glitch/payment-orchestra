//! Operator Service — BC-01 Operator Management
//! 
//! Handles operator (merchant) registration, email verification,
//! and lifecycle management. This is the first service to implement
//! as all other services depend on operator identity.

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

use commands::OperatorCommandHandler;
use queries::OperatorQueries;
use repository::InMemoryOperatorRepository;
use api::grpc::OperatorGrpcService;
use platform_proto::operator::operator_service_server::OperatorServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("operator-service", 9001, 9101).await?;

    let repository = InMemoryOperatorRepository::new();
    let command_handler = OperatorCommandHandler::new(repository.clone());
    let queries = OperatorQueries::new(repository);
    let operator_service = OperatorGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Operator service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(OperatorServiceServer::new(operator_service))
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
    info!("Operator service stopped");
    Ok(())
}

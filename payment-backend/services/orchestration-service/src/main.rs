//! Payment Orchestration Service — Core domain.
//! SVC-05: Manages PaymentIntent lifecycle, routing policy, failover.
//! Event-sourced aggregate for PaymentIntent.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use orchestration_service::api::grpc::OrchestrationGrpcService;
use orchestration_service::commands::OrchestrationCommandHandler;
use orchestration_service::queries::OrchestrationQueryHandler;
use orchestration_service::repository::InMemoryOrchestrationRepository;
use platform_proto::orchestration::orchestration_service_server::OrchestrationServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("orchestration-service", 9005, 9105).await?;

    // Initialize in-memory repository (production: SeaORM + PostgreSQL)
    let repo = InMemoryOrchestrationRepository::new();
    let command_handler = OrchestrationCommandHandler::new(repo.clone());
    let query_handler = OrchestrationQueryHandler::new(repo.clone());

    let grpc_addr: SocketAddr = runner.grpc_addr;
    let orchestration_service = OrchestrationGrpcService::new(command_handler, query_handler);

    info!("Orchestration service gRPC server listening on {grpc_addr}");

    let server = Server::builder()
        .add_service(OrchestrationServiceServer::new(orchestration_service))
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
    info!("Orchestration service stopped");
    Ok(())
}

//! Payment Orchestration Service — Core domain.
//! SVC-05: Manages PaymentIntent lifecycle, routing policy, failover.
//! Event-sourced aggregate for PaymentIntent.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server;
use tracing::info;

use orchestration_service::commands::OrchestrationCommandHandler;
use orchestration_service::queries::OrchestrationQueryHandler;
use orchestration_service::repository::InMemoryOrchestrationRepository;
use orchestration_service::pipeline::OrchestrationPipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("orchestration-service", 9005, 9105).await?;

    // Initialize in-memory repository (production: SeaORM + PostgreSQL)
    let repo = InMemoryOrchestrationRepository::new();
    let command_handler = OrchestrationCommandHandler::new(repo.clone());
    let query_handler = OrchestrationQueryHandler::new(repo.clone());
    let _pipeline = OrchestrationPipeline::new(command_handler, query_handler);

    info!("Orchestration service gRPC server listening on {}", runner.grpc_addr);

    // Keep the process alive until shutdown signal
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Orchestration service stopped");
    Ok(())
}

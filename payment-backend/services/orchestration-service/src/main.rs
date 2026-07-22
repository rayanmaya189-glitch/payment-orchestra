//! Payment Orchestration Service — Core domain.
//! SVC-05: Manages PaymentIntent lifecycle, routing policy, failover.
//! Event-sourced aggregate for PaymentIntent.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use orchestration_service::commands::OrchestrationCommandHandler;
use orchestration_service::queries::OrchestrationQueryHandler;
use orchestration_service::repository::InMemoryOrchestrationRepository;
use orchestration_service::pipeline::OrchestrationPipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("orchestration-service starting...");

    // Initialize in-memory repository (production: SeaORM + PostgreSQL)
    let repo = InMemoryOrchestrationRepository::new();

    // Command handler
    let command_handler = OrchestrationCommandHandler::new(repo.clone());

    // Query handler
    let query_handler = OrchestrationQueryHandler::new(repo.clone());

    // Pipeline with logging, metrics, and authz
    let _pipeline = OrchestrationPipeline::new(
        command_handler,
        query_handler,
    );

    tracing::info!("orchestration-service ready — PaymentIntent lifecycle engine initialized");

    // In a modular monolith, the pipeline handle is registered with the API gateway.
    // In a standalone deployment, this would start a gRPC server on port 9021.

    Ok(())
}

//! Reconciliation Service — Settlement matching engine.
//! SVC-09: Settlement batch ingestion, matching, T+N tracking, fee variance, ledger management.

use reconciliation_service::commands::ReconciliationCommandHandler;
use reconciliation_service::queries::ReconciliationQueryHandler;
use reconciliation_service::repository::InMemoryReconciliationRepository;
use reconciliation_service::pipeline::ReconciliationPipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("reconciliation-service starting...");

    // Initialize in-memory repository (production: SeaORM + PostgreSQL)
    let repo = InMemoryReconciliationRepository::new();

    // Command handler
    let command_handler = ReconciliationCommandHandler::new(repo.clone());

    // Query handler
    let query_handler = ReconciliationQueryHandler::new(repo.clone());

    // Pipeline with logging, metrics, and authz
    let _pipeline = ReconciliationPipeline::new(command_handler, query_handler);

    tracing::info!("reconciliation-service ready — settlement matching engine initialized");
    Ok(())
}

//! Reconciliation Service — Settlement matching engine.
//! SVC-09: Settlement batch ingestion, matching, T+N tracking, fee variance, ledger management.

use reconciliation_service::commands::ReconciliationCommandHandler;
use reconciliation_service::queries::ReconciliationQueryHandler;
use reconciliation_service::repository::InMemoryReconciliationRepository;
use reconciliation_service::pipeline::ReconciliationPipeline;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("reconciliation-service", 9009, 9109).await?;

    let repo = InMemoryReconciliationRepository::new();
    let command_handler = ReconciliationCommandHandler::new(repo.clone());
    let query_handler = ReconciliationQueryHandler::new(repo.clone());
    let _pipeline = ReconciliationPipeline::new(command_handler, query_handler);

    info!("Reconciliation service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("Reconciliation service stopped");
    Ok(())
}

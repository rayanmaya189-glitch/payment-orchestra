//! Reconciliation Service — Settlement matching engine.
//! SVC-09: Settlement batch ingestion, matching, T+N tracking, fee variance, ledger management.

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use reconciliation_service::api::grpc::ReconciliationGrpcService;
use reconciliation_service::commands::ReconciliationCommandHandler;
use reconciliation_service::queries::ReconciliationQueryHandler;
use reconciliation_service::repository::InMemoryReconciliationRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("reconciliation-service", 9009, 9109).await?;

    let repo = InMemoryReconciliationRepository::new();
    let command_handler = ReconciliationCommandHandler::new(repo.clone());
    let query_handler = ReconciliationQueryHandler::new(repo.clone());

    let reconciliation_service = ReconciliationGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Reconciliation service listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
            .add_service(platform_proto::reconciliation::reconciliation_service_server::ReconciliationServiceServer::new(reconciliation_service))
            .serve_with_shutdown(grpc_addr, async {
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
    info!("Reconciliation service stopped");
    Ok(())
}

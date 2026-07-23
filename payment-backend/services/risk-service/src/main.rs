//! Risk Service
//! SVC-11: Fraud scoring, rules engine, synchronous scoring

use std::net::SocketAddr;
use tracing::info;

use risk_service::api::grpc::RiskGrpcService;
use risk_service::commands::RiskCommandHandler;
use risk_service::queries::RiskQueryHandler;
use risk_service::repository::InMemoryRiskRepository;

use platform_proto::risk::risk_service_server::RiskServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let repo = InMemoryRiskRepository::new();
    let command_handler = RiskCommandHandler::new(repo.clone());
    let query_handler = RiskQueryHandler::new(repo);
    let grpc_service = RiskGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("risk-service", 9011, 9111).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Risk service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(RiskServiceServer::new(grpc_service))
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
    info!("Risk service stopped");
    Ok(())
}

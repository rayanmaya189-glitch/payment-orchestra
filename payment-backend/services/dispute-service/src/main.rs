//! Dispute Service
//! SVC-10: Chargeback lifecycle, representment, CRUD + events

use std::net::SocketAddr;
use tracing::info;

use dispute_service::api::grpc::DisputeGrpcService;
use dispute_service::commands::DisputeCommandHandler;
use dispute_service::queries::DisputeQueryHandler;
use dispute_service::repository::InMemoryDisputeRepository;

use platform_proto::dispute::dispute_service_server::DisputeServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let repo = InMemoryDisputeRepository::new();
    let command_handler = DisputeCommandHandler::new(repo.clone());
    let query_handler = DisputeQueryHandler::new(repo);
    let grpc_service = DisputeGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("dispute-service", 9010, 9110).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Dispute service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(DisputeServiceServer::new(grpc_service))
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
    info!("Dispute service stopped");
    Ok(())
}

//! Analytics Service
//! BC-15: Metrics, reporting, analytics queries

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use analytics_service::api::grpc::AnalyticsGrpcService;
use analytics_service::commands::AnalyticsCommandHandler;
use analytics_service::queries::AnalyticsQueryHandler;
use analytics_service::repository::InMemoryAnalyticsStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("analytics-service", 9015, 9115).await?;

    let repo = InMemoryAnalyticsStore::new();
    let command_handler = AnalyticsCommandHandler::new(repo.clone());
    let query_handler = AnalyticsQueryHandler::new(repo.clone());

    let analytics_service = AnalyticsGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Analytics service listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
            .add_service(platform_proto::analytics::analytics_service_server::AnalyticsServiceServer::new(analytics_service))
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
    info!("Analytics service stopped");
    Ok(())
}

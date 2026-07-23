//! Subscription Billing Service
//! SVC-08: Subscription lifecycle, dunning, event-sourced

use std::net::SocketAddr;
use tracing::info;

use subscription_service::api::grpc::SubscriptionGrpcService;
use subscription_service::commands::SubscriptionCommandHandler;
use subscription_service::queries::SubscriptionQueryHandler;
use subscription_service::repository::InMemorySubscriptionRepository;

use platform_proto::subscription::subscription_service_server::SubscriptionServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let repo = InMemorySubscriptionRepository::new();
    let command_handler = SubscriptionCommandHandler::new(repo.clone());
    let query_handler = SubscriptionQueryHandler::new(repo);
    let grpc_service = SubscriptionGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("subscription-service", 9008, 9108).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Subscription service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(SubscriptionServiceServer::new(grpc_service))
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
    info!("Subscription service stopped");
    Ok(())
}

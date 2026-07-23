//! Notification Service
//! BC-14: Email/SMS notifications, webhook delivery

use std::net::SocketAddr;
use tracing::info;

use notification_service::api::grpc::NotificationGrpcService;
use notification_service::commands::NotificationCommandHandler;
use notification_service::queries::NotificationQueryHandler;
use notification_service::repository::InMemoryNotificationRepository;

use platform_proto::notification::notification_service_server::NotificationServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let repo = InMemoryNotificationRepository::new();
    let command_handler = NotificationCommandHandler::new(repo.clone());
    let query_handler = NotificationQueryHandler::new(repo);
    let grpc_service = NotificationGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("notification-service", 9014, 9114).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Notification service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(NotificationServiceServer::new(grpc_service))
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
    info!("Notification service stopped");
    Ok(())
}

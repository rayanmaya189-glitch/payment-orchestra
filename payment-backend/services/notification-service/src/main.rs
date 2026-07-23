//! Notification Service
//! BC-14: Email/SMS notifications, webhook delivery
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use notification_service::api::grpc::NotificationGrpcService;
use notification_service::commands::NotificationCommandHandler;
use notification_service::queries::NotificationQueryHandler;
use notification_service::repository::{InMemoryNotificationRepository, InMemoryWebhookRepository};
use platform_proto::notification::notification_service_server::NotificationServiceServer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("NOTIFICATION").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for notification-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for notification-service ({}), using InMemory", e); None }
    };

    let repo = InMemoryNotificationRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("NOTIFICATION_NATS_USERNAME").ok();
        let nats_password = std::env::var("NOTIFICATION_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as notification_svc", url);
                Arc::new(bus)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to NATS ({}), using NoopEventBus", e);
                Arc::new(NoopEventBus)
            }
        }
    } else {
        Arc::new(NoopEventBus)
    };

    let command_handler = NotificationCommandHandler::new(repo.clone());
    let query_handler = NotificationQueryHandler::new(repo);
    let webhook_repo = InMemoryWebhookRepository::new();
    let grpc_service = NotificationGrpcService::new(command_handler, query_handler, webhook_repo);

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

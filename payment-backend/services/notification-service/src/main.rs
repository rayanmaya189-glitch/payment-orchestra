//! Notification Service
//! BC-14: Email/SMS notifications, webhook delivery

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use notification_service::api::grpc::NotificationGrpcService;
use notification_service::commands::{CommandHandler, NotificationCommandHandler};
use notification_service::queries::{NotificationQueryHandler, QueryHandler};
use notification_service::repository::{
    InMemoryNotificationRepository, InMemoryWebhookRepository, PostgresNotificationRepository,
    WebhookRepository,
};
use platform_proto::notification::notification_service_server::NotificationServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("notification-service", 9014, 9114).await?;

    let grpc_service = if let Ok(db) = create_service_pool("NOTIFICATION").await {
        tracing::info!("Connected to PostgreSQL for notification-service");
        let repo = PostgresNotificationRepository::new(db);
        NotificationGrpcService::new(
            Box::new(NotificationCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
            Box::new(NotificationQueryHandler::new(repo.clone())) as Box<dyn QueryHandler>,
            Box::new(repo) as Box<dyn WebhookRepository>,
        )
    } else {
        tracing::warn!("PostgreSQL unavailable for notification-service, using InMemory");
        let repo = InMemoryNotificationRepository::new();
        let webhook_repo = InMemoryWebhookRepository::new();
        NotificationGrpcService::new(
            Box::new(NotificationCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
            Box::new(NotificationQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            Box::new(webhook_repo) as Box<dyn WebhookRepository>,
        )
    };

    let addr: SocketAddr = runner.grpc_addr;
    info!("Notification service listening on {addr}");

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(MetricsLayer::new("notification-service"))
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

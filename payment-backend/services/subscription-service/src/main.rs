//! Subscription Billing Service
//! SVC-08: Subscription lifecycle, dunning, event-sourced
use std::net::SocketAddr;
use tracing::info;

use subscription_service::api::grpc::SubscriptionGrpcService;
use subscription_service::commands::SubscriptionCommandHandler;
use subscription_service::queries::SubscriptionQueryHandler;
use subscription_service::repository::InMemorySubscriptionRepository;
use platform_proto::subscription::subscription_service_server::SubscriptionServiceServer;
use std::sync::Arc;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("SUBSCRIPTION").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for subscription-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for subscription-service ({}), using InMemory", e); None }
    };

    let repo = InMemorySubscriptionRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("SUBSCRIPTION_NATS_USERNAME").ok();
        let nats_password = std::env::var("SUBSCRIPTION_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as subscription_svc", url);
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

//! Subscription Billing Service
//! SVC-08: Subscription lifecycle, dunning, event-sourced

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use subscription_service::api::grpc::SubscriptionGrpcService;
use subscription_service::commands::{CommandHandler, SubscriptionCommandHandler};
use subscription_service::queries::{SubscriptionQueryHandler, QueryHandler};
use subscription_service::repository::{InMemorySubscriptionRepository, PostgresSubscriptionRepository};
use platform_proto::subscription::subscription_service_server::SubscriptionServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("subscription-service", 9008, 9108).await?;

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("SUBSCRIPTION").await {
            tracing::info!("Connected to PostgreSQL for subscription-service");
            let repo = PostgresSubscriptionRepository::new(db);
            (
                Box::new(SubscriptionCommandHandler::new(repo.clone())),
                Box::new(SubscriptionQueryHandler::new(repo)),
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for subscription-service, using InMemory");
            let repo = InMemorySubscriptionRepository::new();
            (
                Box::new(SubscriptionCommandHandler::new(repo.clone())),
                Box::new(SubscriptionQueryHandler::new(repo)),
            )
        };

    let grpc_service = SubscriptionGrpcService::new(command_handler, query_handler);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Subscription service listening on {addr}");

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
            .layer(MetricsLayer::new("subscription-service"))
            .layer(GrcRateLimitLayer::in_memory("subscription-service"))
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
    platform_logging::telemetry::shutdown();
    info!("Subscription service stopped");
    Ok(())
}

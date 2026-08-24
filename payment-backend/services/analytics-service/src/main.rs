//! Analytics Service
//! BC-15: Metrics, reporting, analytics queries

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use analytics_service::api::grpc::AnalyticsGrpcService;
use analytics_service::commands::AnalyticsCommandHandler;
use analytics_service::queries::AnalyticsQueryHandler;
use analytics_service::repository::InMemoryAnalyticsStore;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let _db = match create_service_pool("ANALYTICS").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for analytics-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for analytics-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("analytics-service", 9015, 9115).await?;

    let repo = InMemoryAnalyticsStore::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("ANALYTICS_NATS_USERNAME").ok();
        let nats_password = std::env::var("ANALYTICS_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as analytics_svc", url);
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

    let command_handler = AnalyticsCommandHandler::new(repo.clone());
    let query_handler = AnalyticsQueryHandler::new(repo.clone());

    let analytics_service = AnalyticsGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Analytics service listening on {}", grpc_addr);

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    tokio::select! {
        result = Server::builder()
            .layer(MetricsLayer::new("analytics-service"))
            .layer(GrcRateLimitLayer::in_memory("analytics-service"))
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
    platform_logging::telemetry::shutdown();
    info!("Analytics service stopped");
    Ok(())
}

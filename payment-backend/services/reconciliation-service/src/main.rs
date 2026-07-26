//! Reconciliation Service — Settlement matching engine.
//! SVC-09: Settlement batch ingestion, matching, T+N tracking, fee variance, ledger management.

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use reconciliation_service::api::grpc::ReconciliationGrpcService;
use reconciliation_service::commands::ReconciliationCommandHandler;
use reconciliation_service::queries::ReconciliationQueryHandler;
use reconciliation_service::repository::InMemoryReconciliationRepository;
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

    let _db = match create_service_pool("RECONCILIATION").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for reconciliation-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for reconciliation-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("reconciliation-service", 9009, 9109).await?;

    let repo = InMemoryReconciliationRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("RECONCILIATION_NATS_USERNAME").ok();
        let nats_password = std::env::var("RECONCILIATION_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as reconciliation_svc", url);
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

    let command_handler = ReconciliationCommandHandler::new(repo.clone());
    let query_handler = ReconciliationQueryHandler::new(repo.clone());

    let reconciliation_service = ReconciliationGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Reconciliation service listening on {}", grpc_addr);

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
            .layer(MetricsLayer::new("reconciliation-service"))
            .layer(GrcRateLimitLayer::in_memory("reconciliation-service"))
            .add_service(platform_proto::reconciliation::reconciliation_service_server::ReconciliationServiceServer::new(reconciliation_service))
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
    info!("Reconciliation service stopped");
    Ok(())
}

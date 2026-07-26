//! Saga Coordinator — BC-17 Durable state machine runtime.
//!
//! Coordinates multi-step cross-aggregate workflows with
//! compensation-capable sagas.

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use saga_coordinator::api::{grpc::SagaGrpcService, SagaApi};
use saga_coordinator::commands::{SagaCommandHandler, CommandHandler};
use saga_coordinator::queries::{SagaQueryHandler, QueryHandler};
use saga_coordinator::repository::{InMemorySagaRepository, PostgresSagaRepository};
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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("saga-coordinator", 9016, 9116).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("SAGA_COORDINATOR_NATS_USERNAME").ok();
        let nats_password = std::env::var("SAGA_COORDINATOR_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as saga_coordinator_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("SAGA_COORDINATOR").await {
            tracing::info!("Connected to PostgreSQL for saga-coordinator");
            let repo = PostgresSagaRepository::new(db);
            (
                Box::new(SagaCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(SagaQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for saga-coordinator, using in-memory");
            let repo = InMemorySagaRepository::new();
            (
                Box::new(SagaCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(SagaQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        };

    let api = SagaApi::new(command_handler, query_handler);
    let saga_service = SagaGrpcService::new(api);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Saga Coordinator service registered, listening on {}", grpc_addr);

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
            .layer(MetricsLayer::new("saga-coordinator"))
            .layer(GrcRateLimitLayer::in_memory("saga-coordinator"))
            .add_service(platform_proto::saga::saga_service_server::SagaServiceServer::new(saga_service))
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
    info!("Saga Coordinator service stopped");
    Ok(())
}

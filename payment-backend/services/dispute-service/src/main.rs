//! Dispute Service
//! SVC-10: Chargeback lifecycle, representment, CRUD + events

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use dispute_service::api::grpc::DisputeGrpcService;
use dispute_service::commands::{CommandHandler, DisputeCommandHandler};
use dispute_service::queries::{DisputeQueryHandler, QueryHandler};
use dispute_service::repository::{InMemoryDisputeRepository, PostgresDisputeRepository};
use platform_proto::dispute::dispute_service_server::DisputeServiceServer;
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
        let nats_username = std::env::var("DISPUTE_NATS_USERNAME").ok();
        let nats_password = std::env::var("DISPUTE_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as dispute_svc", url);
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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("dispute-service", 9010, 9110).await?;

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("DISPUTE").await {
            tracing::info!("Connected to PostgreSQL for dispute-service");
            let repo = PostgresDisputeRepository::new(db);
            (
                Box::new(DisputeCommandHandler::new(repo.clone())),
                Box::new(DisputeQueryHandler::new(repo)),
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for dispute-service, using InMemory");
            let repo = InMemoryDisputeRepository::new();
            (
                Box::new(DisputeCommandHandler::new(repo.clone())),
                Box::new(DisputeQueryHandler::new(repo)),
            )
        };

    let grpc_service = DisputeGrpcService::new(command_handler, query_handler);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Dispute service listening on {addr}");

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
            .layer(MetricsLayer::new("dispute-service"))
            .layer(GrcRateLimitLayer::in_memory("dispute-service"))
            .add_service(DisputeServiceServer::new(grpc_service))
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
    info!("Dispute service stopped");
    Ok(())
}

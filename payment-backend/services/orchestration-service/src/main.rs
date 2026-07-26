//! Payment Orchestration Service — Core domain.
//! SVC-05: Manages PaymentIntent lifecycle, routing policy, failover.
//! Event-sourced aggregate for PaymentIntent.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use std::net::SocketAddr;
use tracing::info;

use orchestration_service::api::grpc::OrchestrationGrpcService;
use orchestration_service::commands::{CommandHandler, OrchestrationCommandHandler};
use orchestration_service::queries::{QueryHandler, OrchestrationQueryHandler};
use orchestration_service::repository::InMemoryOrchestrationRepository;
use orchestration_service::repository::PostgresOrchestrationRepository;
use platform_proto::orchestration::orchestration_service_server::OrchestrationServiceServer;

use std::sync::Arc;
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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("orchestration-service", 9005, 9105).await?;

    // Try PostgreSQL connection; fall back to in-memory if unavailable
    let db_pool = create_service_pool("ORCHESTRATION").await.ok();
    let redis_conn: Option<redis::aio::ConnectionManager> = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => {
                match client.get_connection_manager().await {
                    Ok(conn) => {
                        tracing::info!("Connected to Redis for idempotency cache");
                        Some(conn)
                    }
                    Err(e) => {
                        tracing::warn!("Redis connection manager failed ({}), idempotency will be in-memory", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Redis client creation failed ({}), idempotency will be in-memory", e);
                None
            }
        }
    } else {
        None
    };

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("ORCHESTRATION_NATS_USERNAME").ok();
        let nats_password = std::env::var("ORCHESTRATION_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as orchestration_svc", url);
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

    // Choose repository: PostgreSQL-backed if DB available, otherwise in-memory.
    // Use trait objects (Box<dyn>) to handle different concrete repository types.
    let command_handler: Box<dyn CommandHandler>;
    let query_handler: Box<dyn QueryHandler>;

    if let Some(db) = db_pool {
        tracing::info!("Using PostgreSQL-backed repository for orchestration-service");
        let repo = PostgresOrchestrationRepository::new(db, redis_conn);
        command_handler = Box::new(OrchestrationCommandHandler::new(repo.clone()));
        query_handler = Box::new(OrchestrationQueryHandler::new(repo));
    } else {
        tracing::warn!("PostgreSQL unavailable for orchestration-service, using InMemory repository");
        let repo = InMemoryOrchestrationRepository::new();
        command_handler = Box::new(OrchestrationCommandHandler::new(repo.clone()));
        query_handler = Box::new(OrchestrationQueryHandler::new(repo));
    };

    let addr: SocketAddr = runner.grpc_addr;
    let orchestration_service = OrchestrationGrpcService::new(command_handler, query_handler);

    info!("Orchestration service gRPC server listening on {addr}");

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
            .layer(MetricsLayer::new("orchestration-service"))
            .layer(GrcRateLimitLayer::in_memory("orchestration-service"))
            .add_service(OrchestrationServiceServer::new(orchestration_service))
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
    info!("Orchestration service stopped");
    Ok(())
}

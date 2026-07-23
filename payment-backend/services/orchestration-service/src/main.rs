//! Payment Orchestration Service — Core domain.
//! SVC-05: Manages PaymentIntent lifecycle, routing policy, failover.
//! Event-sourced aggregate for PaymentIntent.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.

use std::net::SocketAddr;
use tracing::info;

use orchestration_service::api::grpc::OrchestrationGrpcService;
use orchestration_service::commands::OrchestrationCommandHandler;
use orchestration_service::queries::OrchestrationQueryHandler;
use orchestration_service::repository::InMemoryOrchestrationRepository;
use platform_proto::orchestration::orchestration_service_server::OrchestrationServiceServer;

use std::sync::Arc;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("ORCHESTRATION").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for orchestration-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for orchestration-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("orchestration-service", 9005, 9105).await?;

    // Initialize in-memory repository (production: SeaORM + PostgreSQL)
    let repo = InMemoryOrchestrationRepository::new();

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

    let command_handler = OrchestrationCommandHandler::new(repo.clone());
    let query_handler = OrchestrationQueryHandler::new(repo.clone());

    let addr: SocketAddr = runner.grpc_addr;
    let orchestration_service = OrchestrationGrpcService::new(command_handler, query_handler);

    info!("Orchestration service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
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

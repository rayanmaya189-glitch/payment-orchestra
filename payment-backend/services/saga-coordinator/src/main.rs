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
use saga_coordinator::repository::InMemorySagaRepository;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("SAGA_COORDINATOR").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for saga-coordinator"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for saga-coordinator ({}), skipping", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("saga-coordinator", 9016, 9116).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("SAGA_COORDINATOR_NATS_USERNAME").ok();
        let nats_password = std::env::var("SAGA_COORDINATOR_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as saga_coordinator_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    let repo = InMemorySagaRepository::new();
    let command_handler = SagaCommandHandler::new(repo.clone());
    let query_handler = SagaQueryHandler::new(repo);
    let api = SagaApi::new(
        Box::new(command_handler) as Box<dyn CommandHandler>,
        Box::new(query_handler) as Box<dyn QueryHandler>,
    );
    let saga_service = SagaGrpcService::new(api);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Saga Coordinator service registered, listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
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

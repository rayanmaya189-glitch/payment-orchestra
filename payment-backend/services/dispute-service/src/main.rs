//! Dispute Service
//! SVC-10: Chargeback lifecycle, representment, CRUD + events

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use dispute_service::api::grpc::DisputeGrpcService;
use dispute_service::commands::DisputeCommandHandler;
use dispute_service::queries::DisputeQueryHandler;
use dispute_service::repository::InMemoryDisputeRepository;
use platform_proto::dispute::dispute_service_server::DisputeServiceServer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("DISPUTE").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for dispute-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for dispute-service ({}), using InMemory", e); None }
    };

    let repo = InMemoryDisputeRepository::new();

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

    let command_handler = DisputeCommandHandler::new(repo.clone());
    let query_handler = DisputeQueryHandler::new(repo);
    let grpc_service = DisputeGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("dispute-service", 9010, 9110).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Dispute service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
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

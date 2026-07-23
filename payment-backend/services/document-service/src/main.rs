//! Document Service
//! BC-13: Document upload, OCR pipeline, secure retrieval

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use document_service::api::grpc::DocumentGrpcService;
use document_service::commands::DocumentCommandHandler;
use document_service::queries::DocumentQueryHandler;
use document_service::repository::InMemoryDocumentRepository;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("DOCUMENT").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for document-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for document-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("document-service", 9013, 9113).await?;

    let repo = InMemoryDocumentRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("DOCUMENT_NATS_USERNAME").ok();
        let nats_password = std::env::var("DOCUMENT_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as document_svc", url);
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

    let command_handler = DocumentCommandHandler::new(repo.clone());
    let query_handler = DocumentQueryHandler::new(repo);

    let document_service = DocumentGrpcService::new(command_handler, query_handler);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Document service listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
            .add_service(platform_proto::document::document_service_server::DocumentServiceServer::new(document_service))
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
    info!("Document service stopped");
    Ok(())
}

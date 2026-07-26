//! Document Service
//! BC-13: Document upload, OCR pipeline, secure retrieval

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use document_service::api::grpc::DocumentGrpcService;
use document_service::commands::{DocumentCommandHandler, CommandHandler};
use document_service::queries::{DocumentQueryHandler, QueryHandler};
use document_service::repository::{InMemoryDocumentRepository, PostgresDocumentRepository};
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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("document-service", 9013, 9113).await?;

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

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("DOCUMENT").await {
            tracing::info!("Connected to PostgreSQL for document-service");
            let repo = PostgresDocumentRepository::new(db);
            (
                Box::new(DocumentCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(DocumentQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for document-service, using in-memory");
            let repo = InMemoryDocumentRepository::new();
            (
                Box::new(DocumentCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(DocumentQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        };

    let document_service = DocumentGrpcService::new(command_handler, query_handler);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Document service listening on {}", grpc_addr);

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
            .layer(MetricsLayer::new("document-service"))
            .layer(GrcRateLimitLayer::in_memory("document-service"))
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
    platform_logging::telemetry::shutdown();
    info!("Document service stopped");
    Ok(())
}

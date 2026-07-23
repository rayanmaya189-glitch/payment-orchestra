//! Invoice Service — Invoice lifecycle management.
//! SVC-06: Invoice create, send, cancel, payment tracking, overdue management.

use std::net::SocketAddr;
use tracing::info;

use invoice_service::api::grpc::InvoiceGrpcService;
use invoice_service::commands::InvoiceCommandHandler;
use invoice_service::queries::InvoiceQueryHandler;
use invoice_service::repository::InMemoryInvoiceRepository;
use platform_proto::invoice::invoice_service_server::InvoiceServiceServer;

use std::sync::Arc;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("INVOICE").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for invoice-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for invoice-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("invoice-service", 9006, 9106).await?;

    let repo = InMemoryInvoiceRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("INVOICE_NATS_USERNAME").ok();
        let nats_password = std::env::var("INVOICE_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as invoice_svc", url);
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

    let command_handler = InvoiceCommandHandler::new(repo.clone());
    let query_handler = InvoiceQueryHandler::new(repo.clone());

    let addr: SocketAddr = runner.grpc_addr;
    let invoice_service = InvoiceGrpcService::new(command_handler, query_handler);

    info!("Invoice service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(InvoiceServiceServer::new(invoice_service))
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
    info!("Invoice service stopped");
    Ok(())
}

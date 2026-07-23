//! Invoice Service — Invoice lifecycle management.
//! SVC-06: Invoice create, send, cancel, payment tracking, overdue management.

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use invoice_service::api::grpc::InvoiceGrpcService;
use invoice_service::commands::InvoiceCommandHandler;
use invoice_service::queries::InvoiceQueryHandler;
use invoice_service::repository::InMemoryInvoiceRepository;
use platform_proto::invoice::invoice_service_server::InvoiceServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("invoice-service", 9006, 9106).await?;

    let repo = InMemoryInvoiceRepository::new();
    let command_handler = InvoiceCommandHandler::new(repo.clone());
    let query_handler = InvoiceQueryHandler::new(repo.clone());

    let grpc_addr: SocketAddr = runner.grpc_addr;
    let invoice_service = InvoiceGrpcService::new(command_handler, query_handler);

    info!("Invoice service gRPC server listening on {grpc_addr}");

    let server = Server::builder()
        .add_service(InvoiceServiceServer::new(invoice_service))
        .serve(grpc_addr);

    tokio::select! {
        result = server => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    info!("Invoice service stopped");
    Ok(())
}

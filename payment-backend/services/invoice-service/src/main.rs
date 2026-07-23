//! Invoice Service — Invoice lifecycle management.
//! SVC-06: Invoice create, send, cancel, payment tracking, overdue management.

use tokio::signal;
use tonic::transport::Server;
use tracing::info;

use invoice_service::commands::InvoiceCommandHandler;
use invoice_service::queries::InvoiceQueryHandler;
use invoice_service::repository::InMemoryInvoiceRepository;
use invoice_service::pipeline::InvoicePipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("invoice-service", 9006, 9106).await?;

    let repo = InMemoryInvoiceRepository::new();
    let command_handler = InvoiceCommandHandler::new(repo.clone());
    let query_handler = InvoiceQueryHandler::new(repo.clone());
    let _pipeline = InvoicePipeline::new(command_handler, query_handler);

    info!("Invoice service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Invoice service stopped");
    Ok(())
}

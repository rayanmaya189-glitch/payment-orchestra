//! Invoice Service — Invoice lifecycle management.
//! SVC-06: Invoice create, send, cancel, payment tracking, overdue management.

use invoice_service::commands::InvoiceCommandHandler;
use invoice_service::queries::InvoiceQueryHandler;
use invoice_service::repository::InMemoryInvoiceRepository;
use invoice_service::pipeline::InvoicePipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("invoice-service starting...");

    let repo = InMemoryInvoiceRepository::new();
    let command_handler = InvoiceCommandHandler::new(repo.clone());
    let query_handler = InvoiceQueryHandler::new(repo.clone());
    let _pipeline = InvoicePipeline::new(command_handler, query_handler);

    tracing::info!("invoice-service ready — invoice lifecycle engine initialized");
    Ok(())
}

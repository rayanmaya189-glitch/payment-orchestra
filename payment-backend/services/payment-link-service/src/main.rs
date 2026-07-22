//! Payment Link Service
//! : SVC-07: Hosted payment links, CRUD + events

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("payment-link-service starting...");
    Ok(())
}

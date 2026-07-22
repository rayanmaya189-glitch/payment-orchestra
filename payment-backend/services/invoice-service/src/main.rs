//! Invoice Service
//! : SVC-06: Invoice lifecycle, event-sourced

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("invoice-service starting...");
    Ok(())
}

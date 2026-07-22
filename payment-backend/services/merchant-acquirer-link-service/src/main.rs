//! Merchant Acquirer Link Service
//! : SVC-21: BYOK Core, credential lifecycle

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("merchant-acquirer-link-service starting...");
    Ok(())
}

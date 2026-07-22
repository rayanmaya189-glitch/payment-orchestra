//! Merchant Connector Onboarding — BYOK onboarding orchestration.
//! SVC-22: Connector selection, credential entry, validation, and activation workflow.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("merchant-connector-onboarding starting...");
    Ok(())
}

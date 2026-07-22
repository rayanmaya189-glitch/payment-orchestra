//! Subscription Billing Service
//! : SVC-08: Subscription lifecycle, dunning, event-sourced

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("subscription-service starting...");
    Ok(())
}

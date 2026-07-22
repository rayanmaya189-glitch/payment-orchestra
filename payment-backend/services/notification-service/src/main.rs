//! Notification Service
//! : SVC-14: Email/SMS/webhook dispatch, event consumer

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("notification-service starting...");
    Ok(())
}

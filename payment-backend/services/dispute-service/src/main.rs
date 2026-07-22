//! Dispute Service
//! : SVC-10: Chargeback lifecycle, representment, CRUD + events

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("dispute-service starting...");
    Ok(())
}

//! Outbox Relay — Background task that polls outbox and publishes to in-process NATS channels.
//! Implements ADR-011 (Transactional Outbox) within each event-sourced service.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("outbox-relay starting...");
    Ok(())
}

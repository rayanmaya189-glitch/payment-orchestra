//! Payment Link Service
//! BC-07: Hosted payment links, CRUD + events

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("payment-link-service starting...");

    // In a full deployment, this would:
    // 1. Connect to PostgreSQL, Redis, NATS
    // 2. Initialize the repository (SeaORM-based)
    // 3. Start gRPC server
    // 4. Register health checks

    tracing::info!("payment-link-service ready");
    Ok(())
}

//! Analytics Service
//! : SVC-15: Event consumer, ClickHouse queries, dashboards

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("analytics-service starting...");
    Ok(())
}

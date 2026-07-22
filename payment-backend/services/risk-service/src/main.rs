//! Risk Service
//! : SVC-11: Fraud scoring, rules engine, synchronous scoring

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("risk-service starting...");
    Ok(())
}

//! Identity & Access Management Service.
//! SVC-02: Authentication, ABAC policy evaluation, token management.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("iam-service starting...");
    Ok(())
}

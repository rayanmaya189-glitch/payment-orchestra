//! Compliance Service
//! : SVC-03: KYB case management, AML screening

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("compliance-service starting...");
    Ok(())
}

//! Document Service
//! : SVC-13: Document management, MinIO storage, OCR pipeline

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();
    tracing::info!("document-service starting...");
    Ok(())
}

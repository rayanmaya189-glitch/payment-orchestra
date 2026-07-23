//! Document Service
//! BC-13: Document upload, OCR pipeline, secure retrieval

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("document-service", 9013, 9113).await?;

    info!("Document service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Document service stopped");
    Ok(())
}

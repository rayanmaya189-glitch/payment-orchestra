//! Document Service
//! BC-13: Document upload, OCR pipeline, secure retrieval

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("document-service", 9013, 9113).await?;

    info!("Document service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Document service stopped");
    Ok(())
}

//! AI Assistant Service
//! BC-12: RAG pipeline, natural-language Q&A, temporal queries

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("ai-assistant-service", 9012, 9112).await?;

    info!("AI Assistant service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&mut runner).await?;
    runner.deregister().await;;

    info!("AI Assistant service stopped");
    Ok(())
}

//! Notification Service
//! BC-14: Email/SMS notifications, webhook delivery

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("notification-service", 9014, 9114).await?;

    info!("Notification service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Notification service stopped");
    Ok(())
}

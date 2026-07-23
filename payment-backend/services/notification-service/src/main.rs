//! Notification Service
//! BC-14: Email/SMS notifications, webhook delivery

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("notification-service", 9014, 9114).await?;

    info!("Notification service registered, listening on {}", runner.grpc_addr);
    runner.wait_for_shutdown().await?;
    runner.deregister().await;;

    info!("Notification service stopped");
    Ok(())
}

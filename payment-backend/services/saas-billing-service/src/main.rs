//! SaaS Billing Service
//! Multi-tenant subscription management, usage tracking, and billing.

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use saas_billing_service::commands::{CommandHandler, SaasBillingCommandHandler};
use saas_billing_service::repository::InMemorySaasBillingRepository;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    // For now, use in-memory repository
    // In production, this would use PostgreSQL
    let repo = InMemorySaasBillingRepository::new();
    repo.seed_test_data().await;

    // Create command handler with in-memory repositories
    let command_handler = SaasBillingCommandHandler::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );

    let _command_handler: Box<dyn CommandHandler> = Box::new(command_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("saas-billing-service", 9009, 9109).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("SaaS Billing service listening on {addr}");

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // For now, just serve a health check
    // In production, this would serve gRPC endpoints
    tokio::select! {
        _ = async {
            // Placeholder: In production, add gRPC service here
            info!("SaaS Billing service started successfully");
            tokio::signal::ctrl_c().await.ok();
        } => {}
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    platform_logging::telemetry::shutdown();
    info!("SaaS Billing service stopped");
    Ok(())
}

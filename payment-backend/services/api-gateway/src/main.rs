//! API Gateway
//! External ingress: REST paths + protobuf bodies, auth, rate limiting

use std::sync::Arc;
use tracing::info;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let _db = match create_service_pool("API_GATEWAY").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for api-gateway"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for api-gateway ({}), skipping", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("api-gateway", 9020, 9120).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("API_GATEWAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("API_GATEWAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as api_gateway_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Initialize gRPC clients for downstream services
    // (used when routing external requests to internal services)
    let iam_addr = format!("http://127.0.0.1:{}", std::env::var("iam_service_grpc_port").unwrap_or_else(|_| "9002".into()));
    let _iam_client = platform_clients::iam::IamClient::connect(&iam_addr).await?;
    tracing::info!(addr = %iam_addr, "IAM gRPC client connected");

    let orchestration_addr = format!("http://127.0.0.1:{}", std::env::var("orchestration_service_grpc_port").unwrap_or_else(|_| "9005".into()));
    let _orchestration_client = platform_clients::orchestration::OrchestrationClient::connect(&orchestration_addr).await?;
    tracing::info!(addr = %orchestration_addr, "Orchestration gRPC client connected");

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    info!("API Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("API Gateway service stopped");
    Ok(())
}

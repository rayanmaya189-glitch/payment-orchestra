//! Merchant Connector Onboarding
//! BYOK flow: create link, test connection, activate

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

    let _db = match create_service_pool("MERCHANT_CONNECTOR_ONBOARDING").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for merchant-connector-onboarding"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for merchant-connector-onboarding ({}), skipping", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("merchant-connector-onboarding", 9022, 9122).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("MERCHANT_CONNECTOR_ONBOARDING_NATS_USERNAME").ok();
        let nats_password = std::env::var("MERCHANT_CONNECTOR_ONBOARDING_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as merchant_connector_onboarding_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    info!("Merchant Connector Onboarding service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;
    platform_logging::telemetry::shutdown();

    info!("Merchant Connector Onboarding service stopped");
    Ok(())
}

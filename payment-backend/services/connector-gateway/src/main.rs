//! Connector Gateway
//! BC-04: Acquirer connector framework, circuit breaker, adapters

use std::sync::Arc;
use tracing::info;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("CONNECTOR_GATEWAY").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for connector-gateway"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for connector-gateway ({}), skipping", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("connector-gateway", 9004, 9104).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("CONNECTOR_GATEWAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("CONNECTOR_GATEWAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as connector_gateway_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    info!("Connector Gateway service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Connector Gateway service stopped");
    Ok(())
}

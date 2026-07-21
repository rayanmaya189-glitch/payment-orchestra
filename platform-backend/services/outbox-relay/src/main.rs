#![allow(dead_code)]
//! Outbox Relay — standalone service that polls unpublished outbox entries
//! and publishes them to NATS JetStream (SRS OUTBOX-001).
//!
//! Provides crash recovery: if a service crashes after writing to the outbox
//! table but before publishing to NATS, the relay picks up the unpublished
//! entry and publishes it.

use platform_config::AppConfig;
use platform_logging::ServiceLogger;

#[tokio::main]
async fn main() {
    ServiceLogger::init("outbox-relay");

    let config = AppConfig::from_env_or_panic("outbox-relay");

    let db = platform_db::connect_database(&config.database).await;

    let nats_url = &config.nats.url;
    let stream_name = &config.nats.stream_name;

    let publisher = platform_messaging::EventPublisher::new(nats_url, stream_name)
        .await
        .expect("Failed to connect to NATS JetStream");

    let relay = platform_messaging::OutboxRelay::new(publisher);

    tracing::info!("Outbox relay starting — polling every 1s for unpublished events");

    if let Err(e) = relay.run(&db).await {
        tracing::error!("Outbox relay fatal error: {e}");
        std::process::exit(1);
    }
}

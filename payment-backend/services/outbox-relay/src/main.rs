//! Outbox Relay — Background task that polls outbox and publishes to NATS channels.
//!
//! Implements ADR-011 (Transactional Outbox) pattern.
//!
//! ## Algorithm
//!
//! 1. Poll `outbox` table for entries with `published = false`
//! 2. Load each entry and publish to NATS
//! 3. On success: mark entry as published
//! 4. On failure: log error and retry on next poll cycle

use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use chrono::Utc;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_outbox::outbox::OutboxEntry;

mod domain;
mod entities;
mod commands;
mod queries;
mod events;
mod repository;
mod api;
mod pipeline;

#[cfg(test)]
mod tests;

use domain::{OutboxRelayConfig, OutboxRelayError, RelayMetrics};
use repository::{
    OutboxRepository, InMemoryOutboxRepository, PostgresOutboxRepository,
};

/// Default outbox relay configuration.
fn default_config() -> OutboxRelayConfig {
    OutboxRelayConfig {
        poll_interval_ms: std::env::var("OUTBOX_RELAY_POLL_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        batch_size: std::env::var("OUTBOX_RELAY_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        max_retries: std::env::var("OUTBOX_RELAY_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
    }
}

/// Main outbox relay loop — polls unpublished entries and publishes them to NATS.
async fn relay_loop(
    repo: Arc<dyn OutboxRepository>,
    event_bus: Arc<dyn EventBus>,
    config: OutboxRelayConfig,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(config.poll_interval_ms));
    let mut metrics = RelayMetrics {
        total_polled: 0,
        total_published: 0,
        total_failed: 0,
        total_duplicates_skipped: 0,
        last_polled_at: None,
        queue_depth: 0,
        is_running: true,
    };

    info!(
        poll_interval_ms = config.poll_interval_ms,
        batch_size = config.batch_size,
        max_retries = config.max_retries,
        "Outbox relay loop started"
    );

    loop {
        interval.tick().await;
        let poll_start = Utc::now();

        // Step 1: Fetch unpublished entries from the outbox
        let entries = match repo.find_unpublished(config.batch_size).await {
            Ok(entries) => entries,
            Err(e) => {
                error!(error = %e, "Failed to fetch unpublished outbox entries");
                continue;
            }
        };

        let batch_count = entries.len() as u32;
        metrics.total_polled += batch_count as u64;
        metrics.last_polled_at = Some(Utc::now());

        if batch_count == 0 {
            continue;
        }

        let mut published = 0u32;
        let mut failed = 0u32;

        // Step 2: Process each entry
        for entry in entries {
            match process_and_publish(&repo, &event_bus, entry).await {
                Ok(()) => published += 1,
                Err(e) => {
                    failed += 1;
                    error!(error = %e, "Failed to publish outbox entry");
                }
            }
        }

        // Step 3: Update metrics
        metrics.total_published += published as u64;
        metrics.total_failed += failed as u64;

        let poll_duration_ms = (Utc::now() - poll_start).num_milliseconds();

        if batch_count > 0 {
            info!(
                batch_size = batch_count,
                published = published,
                failed = failed,
                duration_ms = poll_duration_ms,
                "Outbox relay batch processed"
            );
        }

        // Report queue depth periodically
        if let Ok(depth) = repo.count_unpublished().await {
            metrics.queue_depth = depth;
            if depth > 1000 {
                warn!(queue_depth = depth, "Outbox queue depth is high");
            }
        }
    }
}

/// Publish a single outbox entry to NATS and mark it as published.
async fn process_and_publish(
    repo: &Arc<dyn OutboxRepository>,
    event_bus: &Arc<dyn EventBus>,
    entry: OutboxEntry,
) -> Result<(), OutboxRelayError> {
    let outbox_id = entry.outbox_id;

    // Build the NATS subject from entry metadata
    let subject = format!(
        "{}.{}.{}",
        entry.aggregate_type,
        entry.event_type,
        entry.event_version
    );

    // Publish to NATS
    match event_bus.publish(&subject, entry.payload.clone()).await {
        Ok(()) => {
            repo.mark_published(outbox_id).await?;
            info!(entry_id = %outbox_id, subject = %subject, "Outbox entry published");
            Ok(())
        }
        Err(e) => {
            error!(
                entry_id = %outbox_id,
                subject = %subject,
                error = %e,
                "Failed to publish outbox entry"
            );
            Err(OutboxRelayError::PublishFailed(e.to_string()))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let config = default_config();

    let db_pool = create_service_pool("OUTBOX_RELAY").await.ok();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("outbox-relay", 9019, 9119).await?;

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("OUTBOX_RELAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("OUTBOX_RELAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as outbox_relay_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Initialize repository
    let relay_repo: Arc<dyn OutboxRepository> = if let Some(db) = db_pool {
        info!("Using PostgreSQL-backed repository for outbox-relay");
        Arc::new(PostgresOutboxRepository::new(db))
    } else {
        warn!("PostgreSQL unavailable, using InMemory repository");
        Arc::new(InMemoryOutboxRepository::new())
    };

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Start the relay loop
    let relay_repo_clone = relay_repo.clone();
    let relay_eb = event_bus.clone();
    tokio::spawn(async move {
        relay_loop(relay_repo_clone, relay_eb, config).await;
    });

    info!("Outbox Relay service registered, listening on {}", runner.grpc_addr);
    platform_health::serve::serve_health(&runner).await?;
    runner.deregister().await;

    info!("Outbox Relay service stopped");
    Ok(())
}

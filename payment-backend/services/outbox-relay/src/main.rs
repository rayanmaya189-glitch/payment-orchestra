//! Outbox Relay — Background task that polls outbox and publishes to NATS channels.
//!
//! Implements ADR-011 (Transactional Outbox) pattern.
//!
//! ## Architecture
//!
//! 1. Background `relay_loop` polls the outbox for unpublished entries
//!    and publishes them to NATS with leader-ejection semantics.
//! 2. A gRPC management service (`OutboxRelayService`) provides operational
//!    RPCs for monitoring: metrics, listing entries, and manual append.

// Scaffold modules (`commands`, `queries`, `events`, `entities`, `pipeline`) have
// intentionally unused types and methods for future service composition.
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::result_large_err)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use chrono::Utc;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_outbox::outbox::OutboxEntry;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

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
use api::grpc::OutboxRelayGrpcService;
use platform_proto::outbox_relay::outbox_relay_service_server::OutboxRelayServiceServer;
use platform_proto::health::health_server::HealthServer;
use platform_health::grpc::HealthService;

// ─── Configuration ──────────────────────────────────────────────────────────

/// Default outbox relay configuration, overridable via env vars.
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

// ─── Relay Loop ─────────────────────────────────────────────────────────────

/// Main outbox relay loop — polls unpublished entries and publishes them to NATS.
/// Accepts a shared `metrics` handle so the gRPC `GetMetrics` RPC can query live data.
async fn relay_loop(
    repo: Arc<dyn OutboxRepository>,
    event_bus: Arc<dyn EventBus>,
    config: OutboxRelayConfig,
    metrics: Arc<tokio::sync::RwLock<RelayMetrics>>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(config.poll_interval_ms));

    // Mark relay as running in shared metrics
    {
        let mut m = metrics.write().await;
        m.is_running = true;
    }

    info!(
        poll_interval_ms = config.poll_interval_ms,
        batch_size = config.batch_size,
        max_retries = config.max_retries,
        "Outbox relay loop started"
    );

    loop {
        interval.tick().await;
        let poll_start = Utc::now();

        let entries = match repo.find_unpublished(config.batch_size).await {
            Ok(entries) => entries,
            Err(e) => {
                error!(error = %e, "Failed to fetch unpublished outbox entries");
                continue;
            }
        };

        let batch_count = entries.len() as u32;
        {
            let mut m = metrics.write().await;
            m.total_polled += batch_count as u64;
            m.last_polled_at = Some(Utc::now());
        }

        if batch_count == 0 {
            continue;
        }

        let mut published = 0u32;
        let mut failed = 0u32;

        for entry in entries {
            match process_and_publish(repo.as_ref(), event_bus.as_ref(), entry).await {
                Ok(()) => published += 1,
                Err(e) => {
                    failed += 1;
                    error!(error = %e, "Failed to publish outbox entry");
                }
            }
        }

        {
            let mut m = metrics.write().await;
            m.total_published += published as u64;
            m.total_failed += failed as u64;
        }

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

        if let Ok(depth) = repo.count_unpublished().await {
            let mut m = metrics.write().await;
            m.queue_depth = depth;
            if depth > 1000 {
                warn!(queue_depth = depth, "Outbox queue depth is high");
            }
        }
    }
}

/// Publish a single outbox entry to NATS and mark it as published.
async fn process_and_publish(
    repo: &dyn OutboxRepository,
    event_bus: &dyn EventBus,
    entry: OutboxEntry,
) -> Result<(), OutboxRelayError> {
    let outbox_id = entry.outbox_id;
    let subject = format!("{}.{}.{}", entry.aggregate_type, entry.event_type, entry.event_version);

    match event_bus.publish(&subject, entry.payload.clone()).await {
        Ok(()) => {
            repo.mark_published(outbox_id).await?;
            info!(entry_id = %outbox_id, subject = %subject, "Outbox entry published");
            Ok(())
        }
        Err(e) => {
            error!(entry_id = %outbox_id, subject = %subject, error = %e, "Failed to publish outbox entry");
            Err(OutboxRelayError::PublishFailed(e.to_string()))
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let config = default_config();
    let mut runner = platform_registry::bootstrap::ServerRunner::new("outbox-relay", 9019, 9119).await?;

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("OUTBOX_RELAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("OUTBOX_RELAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as outbox_relay_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Create shared relay metrics — written by relay_loop, read by GetMetrics RPC
    let relay_metrics: Arc<tokio::sync::RwLock<RelayMetrics>> =
        Arc::new(tokio::sync::RwLock::new(RelayMetrics {
            total_polled: 0, total_published: 0, total_failed: 0,
            total_duplicates_skipped: 0, last_polled_at: None,
            queue_depth: 0, is_running: false,
        }));

    // Initialize repository, build handlers, and create gRPC service
    use crate::commands::OutboxCommandHandler;
    use crate::queries::OutboxQueryHandler;

    let (relay_api, relay_repo): (crate::api::OutboxRelayApi, Arc<dyn OutboxRepository>) = {
        let (repo, ch, qh): (Arc<dyn OutboxRepository>, _, _) =
            if let Ok(db) = create_service_pool("OUTBOX_RELAY").await {
                info!("Using PostgreSQL-backed repository for outbox-relay");
                let repo = PostgresOutboxRepository::new(db);
                let ch: Box<dyn crate::commands::CommandHandler> =
                    Box::new(OutboxCommandHandler::new(repo.clone(), config.clone()));
                let qh: Box<dyn crate::queries::QueryHandler> =
                    Box::new(OutboxQueryHandler::new(
                        repo.clone(), config.clone(), relay_metrics.clone(),
                    ));
                (Arc::new(repo) as Arc<dyn OutboxRepository>, ch, qh)
            } else {
                warn!("PostgreSQL unavailable, using InMemory repository");
                let mem_repo = InMemoryOutboxRepository::new();
                let rw_mem = Arc::new(tokio::sync::RwLock::new(mem_repo));
                let adapter = pipeline::ArcRepoAdapter(rw_mem);
                let ch: Box<dyn crate::commands::CommandHandler> =
                    Box::new(OutboxCommandHandler::new(adapter.clone(), config.clone()));
                let qh: Box<dyn crate::queries::QueryHandler> =
                    Box::new(OutboxQueryHandler::new(
                        adapter.clone(), config.clone(), relay_metrics.clone(),
                    ));
                (Arc::new(adapter) as Arc<dyn OutboxRepository>, ch, qh)
            };
        (crate::api::OutboxRelayApi::new(ch, qh), repo)
    };
    let outbox_service = OutboxRelayGrpcService::new(relay_api);
    let health_service = HealthService::new("outbox-relay".to_string());

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Outbox Relay gRPC server listening on {grpc_addr}");

    // Spawn periodic uptime recording
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Start the relay loop as a background task, passing shared metrics
    let relay_repo_clone = relay_repo.clone();
    let relay_eb = event_bus.clone();
    let relay_metrics_clone = relay_metrics.clone();
    tokio::spawn(async move {
        relay_loop(relay_repo_clone, relay_eb, config, relay_metrics_clone).await;
    });

    // Run the gRPC server with graceful shutdown
    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(MetricsLayer::new("outbox-relay"))
            .layer(GrcRateLimitLayer::in_memory("outbox-relay"))
            .add_service(OutboxRelayServiceServer::new(outbox_service))
            .add_service(HealthServer::new(health_service))
            .serve_with_shutdown(grpc_addr, async {
                tokio::signal::ctrl_c().await.ok();
            }) => {
                if let Err(e) = result {
                    tracing::error!("gRPC server error: {}", e);
                }
            }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    runner.deregister().await;
    platform_logging::telemetry::shutdown();
    info!("Outbox Relay service stopped");
    Ok(())
}

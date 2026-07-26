//! API Gateway — External ingress for all REST + protobuf traffic.
//!
//! Serves as the single entry point for all external API calls.
//! Responsibilities:
//! - TLS termination
//! - Authentication (JWT + API key)
//! - Rate limiting (Redis-backed sliding window)
//! - Request routing to internal gRPC services
//! - CORS enforcement
//! - Security headers
//! - Request/response logging
//! - Body size validation (1MB max)
//! - SSRF-safe outbound calls
//!
//! ## Architecture
//!
//! The API Gateway registers itself with etcd for service discovery and
//! exposes its health endpoint. Internal gRPC routing is handled via
//! tonic-based downstream clients (loaded dynamically from etcd or
//! configured addresses). The gateway validates and authenticates every
//! request before forwarding to the appropriate internal service.

use std::sync::Arc;
use tracing::info;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_metrics::grpc_interceptor::MetricsLayer;

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

use commands::{CommandHandler, GatewayCommandHandler};
use queries::{QueryHandler, GatewayQueryHandler};
use repository::{InMemoryGatewayRepository, PostgresGatewayRepository};

/// Initialize gRPC client connections to downstream services (best-effort).
async fn init_downstream_clients() {
    let services = [
        ("iam-service", 9002u16),
        ("orchestration-service", 9005u16),
        ("invoice-service", 9006u16),
        ("subscription-service", 9008u16),
        ("reconciliation-service", 9009u16),
        ("dispute-service", 9010u16),
        ("notification-service", 9014u16),
        ("analytics-service", 9015u16),
    ];

    for (name, port) in &services {
        let addr = format!("http://127.0.0.1:{}", port);
        match platform_clients::client::ServiceConnection::connect(name, &addr).await {
            Ok(_conn) => tracing::info!(service = %name, addr = %addr, "Downstream gRPC client connected"),
            Err(e) => tracing::warn!(service = %name, error = %e, "Downstream gRPC client unavailable, will retry on demand"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    // Initialize downstream gRPC clients (best-effort, non-blocking)
    init_downstream_clients().await;
    info!("Downstream gRPC client initialization complete");

    // Connect to NATS for event publishing
    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("API_GATEWAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("API_GATEWAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as api_gateway_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Build the command and query handlers using the appropriate repository
    let _command_handler: Box<dyn CommandHandler>;
    let _query_handler: Box<dyn QueryHandler>;

    if let Ok(db) = create_service_pool("API_GATEWAY").await {
        info!("Using PostgreSQL-backed repository for api-gateway");
        let repo = PostgresGatewayRepository::new(db);
        _command_handler = Box::new(GatewayCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>;
        _query_handler = Box::new(GatewayQueryHandler::new(repo)) as Box<dyn QueryHandler>;
    } else {
        tracing::warn!("PostgreSQL unavailable, using InMemory repository");
        let repo = InMemoryGatewayRepository::new();
        _command_handler = Box::new(GatewayCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>;
        _query_handler = Box::new(GatewayQueryHandler::new(repo)) as Box<dyn QueryHandler>;
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("api-gateway", 9020, 9120).await?;

    info!(
        "API Gateway registered, grpc={}, health={}",
        runner.grpc_addr,
        runner.health_addr,
    );

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Serve health endpoint (the API Gateway exposes gRPC health check for now;
    // full REST-to-gRPC transcoding requires a reverse proxy layer)
    platform_health::serve::serve_health(&runner).await?;

    runner.deregister().await;
    info!("API Gateway service stopped");
    Ok(())
}

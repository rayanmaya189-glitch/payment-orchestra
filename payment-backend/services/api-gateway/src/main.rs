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
//! exposes its gRPC health endpoint. Internal gRPC routing is handled via
//! tonic-based downstream clients. A gRPC management API (`GatewayApi`)
//! is wired but not yet exposed as a full proto-defined service — the
//! gateway primarily functions as an HTTP-to-gRPC reverse proxy.

// Scaffold modules contain intentionally unused code for future implementation.
#![allow(
    dead_code,
    clippy::result_large_err,
    clippy::match_like_matches_macro
)]

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
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

use commands::{CommandHandler, GatewayCommandHandler};
use queries::{QueryHandler, GatewayQueryHandler};
use repository::{InMemoryGatewayRepository, PostgresGatewayRepository};
use api::GatewayApi;
use platform_proto::health::health_server::HealthServer;
use platform_health::grpc::HealthService;

// ─── Downstream Client Init ─────────────────────────────────────────────────

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
            Err(e) => tracing::warn!(service = %name, error = %e, "Downstream gRPC client unavailable"),
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("api-gateway", 9020, 9120).await?;
    let grpc_addr: SocketAddr = runner.grpc_addr;

    // Build command and query handlers, wire into GatewayApi
    // Wire command and query handlers into GatewayApi for future use.
    // The GatewayApi is stored as `_api` to avoid unused-variable warnings
    // while keeping the handlers alive for when the REST-to-gRPC proxy is connected.
    let _api = {
        let (ch, qh): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
            if let Ok(db) = create_service_pool("API_GATEWAY").await {
                info!("Using PostgreSQL-backed repository for api-gateway");
                let repo = PostgresGatewayRepository::new(db);
                (
                    Box::new(GatewayCommandHandler::new(repo.clone())),
                    Box::new(GatewayQueryHandler::new(repo)),
                )
            } else {
                tracing::warn!("PostgreSQL unavailable, using InMemory repository");
                let repo = InMemoryGatewayRepository::new();
                (
                    Box::new(GatewayCommandHandler::new(repo.clone())),
                    Box::new(GatewayQueryHandler::new(repo)),
                )
            };
        GatewayApi::new(ch, qh)
    };

    info!("API Gateway registered, grpc={grpc_addr}");

    // Spawn periodic uptime recording
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    // Run gRPC health check server with metrics and rate-limit layers
    let health_service = HealthService::new("api-gateway".to_string());

    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(MetricsLayer::new("api-gateway"))
            .layer({
                let rl_config = platform_config::rate_limit::RateLimitConfig::for_service("api-gateway");
                tracing::info!("Rate limits: service_max={}, client_max={}, window={}s", rl_config.service_max, rl_config.client_max, rl_config.window_secs);
                GrcRateLimitLayer::in_memory("api-gateway")
                    .with_service_limit(rl_config.service_max)
                    .with_client_limit(rl_config.client_max)
                    .with_window_secs(rl_config.window_secs)
            })
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
    info!("API Gateway service stopped");
    Ok(())
}

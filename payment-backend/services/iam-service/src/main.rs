//! Identity & Access Management Service — BC-02
//!
//! Handles authentication (Argon2id + JWT), ABAC authorization,
//! API key lifecycle, MFA enrollment, and Maker/Checker flows.

use std::net::SocketAddr;
use tracing::info;

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

use std::sync::Arc;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use commands::{CommandHandler, IamCommandHandler};
use queries::{QueryHandler, IamQueries};
use repository::{InMemoryIamRepository, PostgresIamRepository};
use api::grpc::IamGrpcService;
use platform_proto::iam::iam_service_server::IamServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();
    
    let mut runner = platform_registry::bootstrap::ServerRunner::new("iam-service", 9002, 9102).await?;

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("IAM_NATS_USERNAME").ok();
        let nats_password = std::env::var("IAM_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                info!("Connected to NATS at {} as iam_svc", url);
                Arc::new(bus)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to NATS ({}), using NoopEventBus", e);
                Arc::new(NoopEventBus)
            }
        }
    } else {
        Arc::new(NoopEventBus)
    };

    // Choose repository: PostgreSQL-backed if DB available, otherwise in-memory
    let (command_handler, queries): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("IAM").await {
            tracing::info!("Using PostgreSQL-backed repository for iam-service");
            let repo = PostgresIamRepository::new(db);
            let ch = IamCommandHandler::new(repo.clone(), config.jwt_secret)
                .with_event_bus(event_bus);
            let qh = IamQueries::new(repo);
            (Box::new(ch), Box::new(qh))
        } else {
            tracing::warn!("PostgreSQL unavailable for iam-service, using InMemory repository");
            let repo = InMemoryIamRepository::new();
            let ch = IamCommandHandler::new(repo.clone(), config.jwt_secret)
                .with_event_bus(event_bus);
            let qh = IamQueries::new(repo);
            (Box::new(ch), Box::new(qh))
        };

    let iam_service = IamGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("IAM service gRPC server listening on {addr}");

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(MetricsLayer::new("iam-service"))
            .layer(GrcRateLimitLayer::in_memory("iam-service"))
            .add_service(IamServiceServer::new(iam_service))
            .serve_with_shutdown(addr, async {
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
    info!("IAM service stopped");
    Ok(())
}

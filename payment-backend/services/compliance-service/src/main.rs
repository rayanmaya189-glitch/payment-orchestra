//! Compliance Service — BC-03 Merchant Compliance
//!
//! Handles KYB (Know Your Business) case management,
//! AML transaction monitoring, and SAR report generation.

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
use commands::{CommandHandler, ComplianceCommandHandler};
use queries::{QueryHandler, ComplianceQueries};
use repository::{InMemoryComplianceRepository, PostgresComplianceRepository};
use api::grpc::ComplianceGrpcService;
use platform_proto::compliance::compliance_service_server::ComplianceServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();
    
    let mut runner = platform_registry::bootstrap::ServerRunner::new("compliance-service", 9003, 9103).await?;

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("COMPLIANCE_NATS_USERNAME").ok();
        let nats_password = std::env::var("COMPLIANCE_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                info!("Connected to NATS at {} as compliance_svc", url);
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
        if let Ok(db) = create_service_pool("COMPLIANCE").await {
            info!("Using PostgreSQL-backed repository for compliance-service");
            let repo = PostgresComplianceRepository::new(db);
            let ch = ComplianceCommandHandler::new(repo.clone()).with_event_bus(event_bus);
            let qh = ComplianceQueries::new(repo);
            (Box::new(ch), Box::new(qh))
        } else {
            tracing::warn!("PostgreSQL unavailable for compliance-service, using InMemory repository");
            let repo = InMemoryComplianceRepository::new();
            let ch = ComplianceCommandHandler::new(repo.clone()).with_event_bus(event_bus);
            let qh = ComplianceQueries::new(repo);
            (Box::new(ch), Box::new(qh))
        };

    let compliance_service = ComplianceGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Compliance service gRPC server listening on {addr}");

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
            .layer(MetricsLayer::new("compliance-service"))
            .layer(GrcRateLimitLayer::in_memory("compliance-service"))
            .add_service(ComplianceServiceServer::new(compliance_service))
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
    info!("Compliance service stopped");
    Ok(())
}

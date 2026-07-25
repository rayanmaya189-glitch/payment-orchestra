//! Operator Service — BC-01 Operator Management
//! 
//! Handles operator (merchant) registration, email verification,
//! and lifecycle management. This is the first service to implement
//! as all other services depend on operator identity.

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
use commands::{CommandHandler, OperatorCommandHandler};
use queries::{OperatorQueries, QueryHandler};
use repository::{InMemoryOperatorRepository, PostgresOperatorRepository};
use api::grpc::OperatorGrpcService;
use platform_proto::operator::operator_service_server::OperatorServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();
    

    let _db = match create_service_pool("OPERATOR").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for operator-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for operator-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("operator-service", 9001, 9101).await?;

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("OPERATOR_NATS_USERNAME").ok();
        let nats_password = std::env::var("OPERATOR_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                info!("Connected to NATS at {} as operator_svc", url);
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

    let (command_handler, queries): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("OPERATOR").await {
            tracing::info!("Connected to PostgreSQL for operator-service");
            let repo = PostgresOperatorRepository::new(db);
            (
                Box::new(OperatorCommandHandler::new(repo.clone()).with_event_bus(event_bus.clone())),
                Box::new(OperatorQueries::new(repo)),
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for operator-service, using InMemory");
            let repo = InMemoryOperatorRepository::new();
            (
                Box::new(OperatorCommandHandler::new(repo.clone()).with_event_bus(event_bus)),
                Box::new(OperatorQueries::new(repo)),
            )
        };

    let operator_service = OperatorGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Operator service gRPC server listening on {addr}");

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
            .layer(MetricsLayer::new("operator-service"))
            .add_service(OperatorServiceServer::new(operator_service))
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
    info!("Operator service stopped");
    Ok(())
}

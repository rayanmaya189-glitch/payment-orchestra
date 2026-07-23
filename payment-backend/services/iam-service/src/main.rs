//! Identity & Access Management Service — BC-02
//!
//! Handles authentication (Argon2id + JWT), ABAC authorization,
//! API key lifecycle, MFA enrollment, and Maker/Checker flows.

use std::net::SocketAddr;
use tracing::info;

mod domain;
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
use commands::IamCommandHandler;
use queries::IamQueries;
use repository::InMemoryIamRepository;
use api::grpc::IamGrpcService;
use platform_proto::iam::iam_service_server::IamServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("iam-service", 9002, 9102).await?;

    let _db = match create_service_pool("IAM").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for iam-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for iam-service ({}), using InMemory", e); None }
    };

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    let repository = InMemoryIamRepository::new();

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

    let command_handler = IamCommandHandler::new(
        repository.clone(),
        config.jwt_secret,
    ).with_event_bus(event_bus);
    let queries = IamQueries::new(repository);
    let iam_service = IamGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("IAM service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
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

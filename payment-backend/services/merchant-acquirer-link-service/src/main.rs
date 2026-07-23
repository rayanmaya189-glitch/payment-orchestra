//! Merchant Acquirer Link Service — BYOK Core
//!
//! Owns the MerchantAcquirerLink aggregate — connects an operator
//! to a specific payment gateway using their own credentials.

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
use commands::LinkCommandHandler;
use queries::LinkQueries;
use repository::InMemoryLinkRepository;
use api::grpc::LinkGrpcService;
use platform_proto::connector::merchant_acquirer_link_service_server::MerchantAcquirerLinkServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let _db = match create_service_pool("MERCHANT_ACQUIRER_LINK").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for merchant-acquirer-link-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for merchant-acquirer-link-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("merchant-acquirer-link-service", 9018, 9118).await?;

    let repository = InMemoryLinkRepository::new();

    let event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("MERCHANT_ACQUIRER_LINK_NATS_USERNAME").ok();
        let nats_password = std::env::var("MERCHANT_ACQUIRER_LINK_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                info!("Connected to NATS at {} as merchant_acquirer_link_svc", url);
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

    let command_handler = LinkCommandHandler::new(repository.clone())
        .with_event_bus(event_bus);
    let queries = LinkQueries::new(repository);
    let link_service = LinkGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Merchant Acquirer Link service gRPC server listening on {addr}");

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(MerchantAcquirerLinkServiceServer::new(link_service))
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
    info!("Merchant Acquirer Link service stopped");
    Ok(())
}

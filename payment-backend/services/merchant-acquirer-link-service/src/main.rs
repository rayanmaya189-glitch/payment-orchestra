//! Merchant Acquirer Link Service — BYOK Core
//!
//! Owns the MerchantAcquirerLink aggregate — connects an operator
//! to a specific payment gateway using their own credentials.

use std::net::SocketAddr;
use std::sync::Arc;
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

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use commands::{CommandHandler, LinkCommandHandler};
use queries::{LinkQueries, QueryHandler};
use repository::{InMemoryLinkRepository, PostgresLinkRepository};
use api::grpc::LinkGrpcService;
use platform_proto::connector::merchant_acquirer_link_service_server::MerchantAcquirerLinkServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("merchant-acquirer-link-service", 9018, 9118).await?;

    let (command_handler, queries): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("MERCHANT_ACQUIRER_LINK").await {
            tracing::info!("Connected to PostgreSQL for merchant-acquirer-link-service");
            let repo = PostgresLinkRepository::new(db);
            (
                Box::new(LinkCommandHandler::new(repo.clone()).with_event_bus(event_bus.clone())),
                Box::new(LinkQueries::new(repo)),
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for merchant-acquirer-link-service, using InMemory");
            let repo = InMemoryLinkRepository::new();
            (
                Box::new(LinkCommandHandler::new(repo.clone()).with_event_bus(event_bus)),
                Box::new(LinkQueries::new(repo)),
            )
        };

    let link_service = LinkGrpcService::new(command_handler, queries);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Merchant Acquirer Link service gRPC server listening on {addr}");

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
            .layer(MetricsLayer::new("merchant-acquirer-link-service"))
            .layer(GrcRateLimitLayer::in_memory("merchant-acquirer-link-service"))
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

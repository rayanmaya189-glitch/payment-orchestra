//! Merchant Acquirer Link Service — BYOK Core
//!
//! Owns the MerchantAcquirerLink aggregate — connects an operator
//! to a specific payment gateway using their own credentials.

use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server;
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

use commands::LinkCommandHandler;
use queries::LinkQueries;
use repository::InMemoryLinkRepository;
use api::grpc::LinkGrpcService;
use platform_proto::connector::merchant_acquirer_link_service_server::MerchantAcquirerLinkServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform_logging::telemetry::init();

    let config = platform_config::config::ServiceConfig::from_env()
        .unwrap_or_default();

    info!(
        service = %config.service_name,
        listen_addr = %config.listen_addr,
        "Merchant Acquirer Link service starting"
    );

    let repository = InMemoryLinkRepository::new();
    let command_handler = LinkCommandHandler::new(repository.clone());
    let queries = LinkQueries::new(repository);

    let link_service = LinkGrpcService::new(command_handler, queries);

    let addr: SocketAddr = config.listen_addr.parse()
        .unwrap_or_else(|_| "0.0.0.0:9004".parse().unwrap());

    info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(MerchantAcquirerLinkServiceServer::new(link_service))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c().await.ok();
            info!("Shutdown signal received");
        })
        .await?;

    info!("Merchant Acquirer Link service stopped");
    Ok(())
}

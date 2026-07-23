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

use commands::LinkCommandHandler;
use queries::LinkQueries;
use repository::InMemoryLinkRepository;
use api::grpc::LinkGrpcService;
use platform_proto::connector::merchant_acquirer_link_service_server::MerchantAcquirerLinkServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    

    let mut runner = platform_registry::bootstrap::ServerRunner::new("merchant-acquirer-link-service", 9018, 9118).await?;

    let repository = InMemoryLinkRepository::new();
    let command_handler = LinkCommandHandler::new(repository.clone());
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

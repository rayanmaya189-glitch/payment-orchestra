//! Payment Link Service
//! BC-07: Hosted payment links, CRUD + events

use std::net::SocketAddr;
use tracing::info;

use payment_link_service::api::grpc::PaymentLinkGrpcService;
use payment_link_service::commands::PaymentLinkCommandHandler;
use payment_link_service::queries::PaymentLinkQueryHandler;
use payment_link_service::repository::InMemoryPaymentLinkRepository;

use platform_proto::payment_link::payment_link_service_server::PaymentLinkServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let repo = InMemoryPaymentLinkRepository::new();
    let command_handler = PaymentLinkCommandHandler::new(repo.clone());
    let query_handler = PaymentLinkQueryHandler::new(repo);
    let grpc_service = PaymentLinkGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("payment-link-service", 9007, 9107).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Payment Link service listening on {}", addr);

    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(PaymentLinkServiceServer::new(grpc_service))
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
    info!("Payment Link service stopped");
    Ok(())
}

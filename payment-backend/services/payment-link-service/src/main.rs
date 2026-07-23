//! Payment Link Service
//! BC-07: Hosted payment links, CRUD + events
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use payment_link_service::api::grpc::PaymentLinkGrpcService;
use payment_link_service::commands::PaymentLinkCommandHandler;
use payment_link_service::queries::PaymentLinkQueryHandler;
use payment_link_service::repository::InMemoryPaymentLinkRepository;
use platform_proto::payment_link::payment_link_service_server::PaymentLinkServiceServer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("PAYMENT_LINK").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for payment-link-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for payment-link-service ({}), using InMemory", e); None }
    };

    let repo = InMemoryPaymentLinkRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("PAYMENT_LINK_NATS_USERNAME").ok();
        let nats_password = std::env::var("PAYMENT_LINK_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as payment_link_svc", url);
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

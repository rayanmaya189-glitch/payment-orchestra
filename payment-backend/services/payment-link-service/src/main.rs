//! Payment Link Service
//! BC-07: Hosted payment links, CRUD + events

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use payment_link_service::api::grpc::PaymentLinkGrpcService;
use payment_link_service::commands::{CommandHandler, PaymentLinkCommandHandler};
use payment_link_service::queries::{PaymentLinkQueryHandler, QueryHandler};
use payment_link_service::repository::{InMemoryPaymentLinkRepository, PostgresPaymentLinkRepository};
use platform_proto::payment_link::payment_link_service_server::PaymentLinkServiceServer;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

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

    let mut runner = platform_registry::bootstrap::ServerRunner::new("payment-link-service", 9007, 9107).await?;

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("PAYMENT_LINK").await {
            tracing::info!("Connected to PostgreSQL for payment-link-service");
            let repo = PostgresPaymentLinkRepository::new(db);
            (
                Box::new(PaymentLinkCommandHandler::new(repo.clone())),
                Box::new(PaymentLinkQueryHandler::new(repo)),
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for payment-link-service, using InMemory");
            let repo = InMemoryPaymentLinkRepository::new();
            (
                Box::new(PaymentLinkCommandHandler::new(repo.clone())),
                Box::new(PaymentLinkQueryHandler::new(repo)),
            )
        };

    let grpc_service = PaymentLinkGrpcService::new(command_handler, query_handler);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Payment Link service listening on {addr}");

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
            .layer(MetricsLayer::new("payment-link-service"))
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

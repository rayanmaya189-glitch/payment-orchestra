//! Risk Service
//! SVC-11: Fraud scoring, rules engine, synchronous scoring
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;
use risk_service::api::grpc::RiskGrpcService;
use risk_service::commands::{RiskCommandHandler, CommandHandler};
use risk_service::queries::{RiskQueryHandler, QueryHandler};
use risk_service::repository::{InMemoryRiskRepository, PostgresRiskRepository};
use platform_proto::risk::risk_service_server::RiskServiceServer;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("RISK_NATS_USERNAME").ok();
        let nats_password = std::env::var("RISK_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as risk_svc", url);
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

    let (command_handler, query_handler): (Box<dyn CommandHandler>, Box<dyn QueryHandler>) =
        if let Ok(db) = create_service_pool("RISK").await {
            tracing::info!("Connected to PostgreSQL for risk-service");
            let repo = PostgresRiskRepository::new(db);
            (
                Box::new(RiskCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(RiskQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        } else {
            tracing::warn!("PostgreSQL unavailable for risk-service, using in-memory");
            let repo = InMemoryRiskRepository::new();
            (
                Box::new(RiskCommandHandler::new(repo.clone())) as Box<dyn CommandHandler>,
                Box::new(RiskQueryHandler::new(repo)) as Box<dyn QueryHandler>,
            )
        };

    let grpc_service = RiskGrpcService::new(command_handler, query_handler);

    let mut runner = platform_registry::bootstrap::ServerRunner::new("risk-service", 9011, 9111).await?;

    let addr: SocketAddr = runner.grpc_addr;
    info!("Risk service listening on {}", addr);

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
            .layer(MetricsLayer::new("risk-service"))
            .layer(GrcRateLimitLayer::in_memory("risk-service"))
            .add_service(RiskServiceServer::new(grpc_service))
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
    info!("Risk service stopped");
    Ok(())
}

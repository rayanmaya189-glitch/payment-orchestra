//! Connector Gateway
//! BC-04: Acquirer connector framework, circuit breaker, adapters, gateway profiles
//!
//! Serves the GatewayProfileService gRPC API for:
//! - Gateway profile management (CRUD)
//! - Connector discovery (list, schema)
//! - Credential operations (validate, test connection)
//! With Redis-backed rate limiting

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use connector_gateway::api::grpc::GatewayProfileGrpcService;
use connector_gateway::api::rate_limit::{RateLimitInterceptor, create_rate_limiter};
use connector_gateway::commands::GatewayCommandHandler;
use connector_gateway::queries::GatewayQueryHandler;
use connector_gateway::repository::InMemoryGatewayProfileRepository;
use connector_gateway::domain::ConnectorRegistry;
use platform_proto::gateway_profile::gateway_profile_service_server::GatewayProfileServiceServer;

use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let _db = match create_service_pool("CONNECTOR_GATEWAY").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for connector-gateway"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for connector-gateway ({}), skipping", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("connector-gateway", 9004, 9104).await?;

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("CONNECTOR_GATEWAY_NATS_USERNAME").ok();
        let nats_password = std::env::var("CONNECTOR_GATEWAY_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(&url, nats_username.as_deref(), nats_password.as_deref()).await {
            Ok(bus) => { tracing::info!("Connected to NATS as connector_gateway_svc"); Arc::new(bus) }
            Err(e) => { tracing::warn!("NATS failed ({}), using NoopEventBus", e); Arc::new(NoopEventBus) }
        }
    } else { Arc::new(NoopEventBus) };

    // Initialize in-memory repository
    let repo = InMemoryGatewayProfileRepository::new();

    // Create command and query handlers with separate registries
    // (ConnectorRegistry is not Clone since it contains Box<dyn> trait objects)
    let command_handler = GatewayCommandHandler::new(
        repo.clone(),
        ConnectorRegistry::with_all_connectors(),
    );
    let query_handler = GatewayQueryHandler::new(
        repo.clone(),
        ConnectorRegistry::with_all_connectors(),
    );

    // Create the gRPC service with its own registry for direct connector access
    let gateway_profile_service = GatewayProfileGrpcService::new(
        command_handler,
        query_handler,
        ConnectorRegistry::with_all_connectors(),
    );

    // Initialize rate limiter (gracefully falls back to no-op if Redis is unavailable)
    let rate_limiter = create_rate_limiter(None).await;
    let rate_limit_interceptor = RateLimitInterceptor::new(rate_limiter);

    let addr: SocketAddr = runner.grpc_addr;
    info!("Connector Gateway gRPC server listening on {addr}");
    info!("Connectors available: stripe, network_international, checkout_com, telr");

    // Start gRPC server with rate limiting interceptor
    tokio::select! {
        result = tonic::transport::Server::builder()
            .layer(tonic::service::interceptor(rate_limit_interceptor))
            .add_service(GatewayProfileServiceServer::new(gateway_profile_service))
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
    info!("Connector Gateway service stopped");
    Ok(())
}

//! AI Assistant Service
//! BC-12: RAG pipeline, natural-language Q&A, temporal queries

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use ai_assistant_service::api::grpc::AiAssistantGrpcService;
use ai_assistant_service::commands::AiCommandHandler;
use ai_assistant_service::queries::AiQueryHandler;
use ai_assistant_service::repository::InMemoryConversationSessionRepository;
use platform_db::connection::create_service_pool;
use platform_messaging::event_bus::{EventBus, NoopEventBus};
use platform_messaging::nats_event_bus::NatsJetStreamEventBus;
use platform_metrics::grpc_interceptor::MetricsLayer;
use platform_middleware::rate_limit::GrcRateLimitLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();
    platform_metrics::init_uptime_tracker();

    let _db = match create_service_pool("AI_ASSISTANT").await {
        Ok(db) => { tracing::info!("Connected to PostgreSQL for ai-assistant-service"); Some(db) }
        Err(e) => { tracing::warn!("PostgreSQL unavailable for ai-assistant-service ({}), using InMemory", e); None }
    };

    let mut runner = platform_registry::bootstrap::ServerRunner::new("ai-assistant-service", 9012, 9112).await?;

    let repo = InMemoryConversationSessionRepository::new();

    let _event_bus: Arc<dyn EventBus> = if let Ok(url) = std::env::var("NATS_URL") {
        let nats_username = std::env::var("AI_ASSISTANT_NATS_USERNAME").ok();
        let nats_password = std::env::var("AI_ASSISTANT_NATS_PASSWORD").ok();
        match NatsJetStreamEventBus::connect_with_auth(
            &url,
            nats_username.as_deref(),
            nats_password.as_deref(),
        ).await {
            Ok(bus) => {
                tracing::info!("Connected to NATS at {} as ai_assistant_svc", url);
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

    let rag_engine = Box::new(ai_assistant_service::commands::SimulatedRagEngine);
    let rate_limiter = Box::new(ai_assistant_service::commands::NoopRateLimiter);
    let command_handler = AiCommandHandler::new(repo.clone(), rag_engine, rate_limiter);
    let query_handler = AiQueryHandler::new(repo.clone());

    let ai_service = AiAssistantGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("AI Assistant service listening on {}", grpc_addr);

    // Spawn periodic uptime recording (30s cadence aligns with Prometheus scrape)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            platform_metrics::record_uptime();
        }
    });

    tokio::select! {
        result = Server::builder()
            .layer(MetricsLayer::new("ai-assistant-service"))
            .layer(GrcRateLimitLayer::in_memory("ai-assistant-service"))
            .add_service(platform_proto::ai_assistant::ai_assistant_service_server::AiAssistantServiceServer::new(ai_service))
            .serve_with_shutdown(grpc_addr, async {
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
    info!("AI Assistant service stopped");
    Ok(())
}

//! AI Assistant Service
//! BC-12: RAG pipeline, natural-language Q&A, temporal queries

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use ai_assistant_service::api::grpc::AiAssistantGrpcService;
use ai_assistant_service::commands::AiCommandHandler;
use ai_assistant_service::queries::AiQueryHandler;
use ai_assistant_service::repository::InMemoryConversationSessionRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("ai-assistant-service", 9012, 9112).await?;

    let repo = InMemoryConversationSessionRepository::new();
    let rag_engine = Box::new(ai_assistant_service::commands::SimulatedRagEngine);
    let rate_limiter = Box::new(ai_assistant_service::commands::NoopRateLimiter);
    let command_handler = AiCommandHandler::new(repo.clone(), rag_engine, rate_limiter);
    let query_handler = AiQueryHandler::new(repo.clone());

    let ai_service = AiAssistantGrpcService::new(command_handler, query_handler, repo);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("AI Assistant service listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
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

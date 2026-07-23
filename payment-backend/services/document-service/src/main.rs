//! Document Service
//! BC-13: Document upload, OCR pipeline, secure retrieval

use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

use document_service::api::grpc::DocumentGrpcService;
use document_service::commands::DocumentCommandHandler;
use document_service::queries::DocumentQueryHandler;
use document_service::repository::InMemoryDocumentRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    platform_logging::telemetry::init();

    let mut runner = platform_registry::bootstrap::ServerRunner::new("document-service", 9013, 9113).await?;

    let repo = InMemoryDocumentRepository::new();
    let command_handler = DocumentCommandHandler::new(repo.clone());
    let query_handler = DocumentQueryHandler::new(repo);

    let document_service = DocumentGrpcService::new(command_handler, query_handler);

    let grpc_addr: SocketAddr = runner.grpc_addr;
    info!("Document service listening on {}", grpc_addr);

    tokio::select! {
        result = Server::builder()
            .add_service(platform_proto::document::document_service_server::DocumentServiceServer::new(document_service))
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
    info!("Document service stopped");
    Ok(())
}

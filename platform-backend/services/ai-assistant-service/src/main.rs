#![allow(dead_code)]
use axum::{routing::get, Router};
use platform_config::AppConfig;
use platform_logging::ServiceLogger;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod api;
mod application;
mod domain;
mod infrastructure;

use crate::application::services::AiAssistantServiceImpl;
use crate::infrastructure::adapters::PostgresQueryRepository;

#[tokio::main]
async fn main() {
    ServiceLogger::init("ai-assistant-service");
    let config = AppConfig::from_env_or_panic("ai-assistant-service");
    let db = infrastructure::database::connect(&config.database).await;
    let redis = infrastructure::cache::connect(&config.redis).await;

    let repo = PostgresQueryRepository::new(db.clone());
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let service = AiAssistantServiceImpl::new(Box::new(repo), db.clone(), ollama_url);

    let app_state = api::AppState::new(service);
    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(redis, platform_middleware::RateLimitLayerConfig::default());

    let app = Router::new()
        .route("/healthz", get(healthz))
        .nest("/v1", api::routes::router(app_state)
            .layer(platform_middleware::JwtAuthLayer::new(config.auth.clone())))
        .layer(platform_middleware::SecurityHeadersLayer)
        .layer(platform_middleware::RequestIdLayer)
        .layer(rate_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    tracing::info!("AI Assistant service listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let shutdown = platform_middleware::shutdown_signal(platform_middleware::ShutdownConfig::standard("ai-assistant-service"));
    axum::serve(listener, app).with_graceful_shutdown(shutdown).await.unwrap();
}

async fn healthz() -> &'static str { "ok" }

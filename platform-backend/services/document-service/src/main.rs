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

use crate::application::services::DocumentServiceImpl;
use crate::infrastructure::adapters::{PostgresDocumentRepository, LocalStorageProvider};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    platform_logging::install_panic_hook();
    ServiceLogger::init("document-service");

    let config = AppConfig::from_env_or_panic("document-service");

    let db = infrastructure::database::connect(&config.database).await;
    let redis = infrastructure::cache::connect(&config.redis).await;

    let repo = PostgresDocumentRepository::new(db.clone());
    let storage = LocalStorageProvider::new(
        std::env::var("DOCUMENT_STORAGE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/document-storage")),
    );
    let service = DocumentServiceImpl::new(Box::new(repo), Box::new(storage), db.clone());

    let app_state = api::AppState::new(service);

    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(
        redis,
        platform_middleware::RateLimitLayerConfig {
            endpoint_overrides: vec![],
            login_per_ip_per_minute: config.rate_limit.login_per_ip_per_minute,
            api_per_principal_per_second: config.rate_limit.api_per_principal_per_second,
        },
    );

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest(
            "/v1",
            api::routes::router(app_state)
                .layer(platform_middleware::JwtAuthLayer::new(config.auth.clone())),
        )
        .layer(platform_middleware::SecurityHeadersLayer)
        .layer(platform_middleware::RequestIdLayer)
        .layer(rate_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("Document service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let shutdown_signal = platform_middleware::shutdown_signal(
        platform_middleware::ShutdownConfig::standard("document-service"),
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "checks": { "postgres": "not_checked" }
    }))
}

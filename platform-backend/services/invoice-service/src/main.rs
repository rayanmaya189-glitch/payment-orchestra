use axum::{routing::get, Router};
use platform_config::AppConfig;
use platform_logging::ServiceLogger;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

mod api;
mod application;
mod domain;
mod infrastructure;

use crate::application::services::InvoiceServiceImpl;
use crate::infrastructure::adapters::PostgresInvoiceRepository;

#[tokio::main]
async fn main() {
    ServiceLogger::init("invoice-service");

    let config = AppConfig::from_env_or_panic("invoice-service");

    let db = infrastructure::database::connect(&config.database).await;
    let redis = infrastructure::cache::connect(&config.redis).await;

    let repo = PostgresInvoiceRepository::new(db.clone());
    let service = InvoiceServiceImpl::new(Box::new(repo));

    let app_state = api::AppState {
        service: Arc::new(service),
    };

    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(
        redis,
        platform_middleware::RateLimitLayerConfig {
            login_per_ip_per_minute: config.rate_limit.login_per_ip_per_minute,
            api_per_principal_per_second: config.rate_limit.api_per_principal_per_second,
        },
    );

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/v1", api::routes::router(app_state)
            .layer(platform_middleware::JwtAuthLayer::new(config.auth.clone())))
        .layer(platform_middleware::SecurityHeadersLayer)
        .layer(platform_middleware::RequestIdLayer)
        .layer(rate_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    tracing::info!("Invoice service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let shutdown_signal = async {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        tracing::info!("Shutdown signal received...");
    };
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal).await.unwrap();
    tracing::info!("Invoice service shut down gracefully");
}

async fn healthz() -> &'static str { "ok" }
async fn readyz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "checks": { "postgres": "not_checked", "redis": "not_checked" }
    }))
}

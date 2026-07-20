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

use crate::application::services::RiskServiceImpl;

#[tokio::main]
async fn main() {
    ServiceLogger::init("risk-service");

    let config = AppConfig::from_env("risk-service")
        .expect("Failed to load config from environment");

    // Connect to Postgres with retry
    let db = infrastructure::database::connect(&config.database).await;

    // Connect to Redis with retry
    let redis = infrastructure::cache::connect(&config.redis).await;

    // Create service
    let service = RiskServiceImpl::new(db.clone());
    let app_state = api::AppState { service: Arc::new(service) };

    // Build shared middleware stack
    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(
        redis.clone(),
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

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("Risk service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        tracing::info!("Shutdown signal received, starting graceful shutdown...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    tracing::info!("Risk service shut down gracefully");
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "checks": { "postgres": "not_checked", "redis": "not_checked" }
    }))
}

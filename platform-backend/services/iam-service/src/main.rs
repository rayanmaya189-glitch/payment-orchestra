#![allow(dead_code)]
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

use crate::application::services::IamServiceImpl;
use crate::infrastructure::adapters::{PostgresPrincipalRepository, PostgresApiKeyRepository, PostgresRefreshTokenRepository};
use crate::infrastructure::cache::RedisSessionStore;

#[tokio::main]
async fn main() {
    platform_logging::install_panic_hook();
    ServiceLogger::init("iam-service");

    let config = AppConfig::from_env_or_panic("iam-service");

    let db = infrastructure::database::connect(&config.database).await;
    let redis = infrastructure::cache::connect(&config.redis).await;

    let principal_repo = PostgresPrincipalRepository::new(db.clone());
    let api_key_repo = PostgresApiKeyRepository::new(db.clone());
    let refresh_token_repo = PostgresRefreshTokenRepository::new(db.clone());
    let session_store = RedisSessionStore::new(redis.clone());

    let service = IamServiceImpl::new(
        Box::new(principal_repo),
        Box::new(api_key_repo),
        Box::new(refresh_token_repo),
        session_store,
        db.clone(),
        config.auth.clone(),
    );

    let app_state = api::AppState::new(service, config.auth.clone());

    // API key lookup for the middleware
    let api_key_lookup: Arc<dyn platform_middleware::auth::ApiKeyLookup> =
        Arc::new(PostgresApiKeyRepository::new(db.clone()));

    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(
        redis,
        platform_middleware::RateLimitLayerConfig {
            endpoint_overrides: vec![
                ("/v1/auth/login".into(), 10, 60),
                ("/v1/auth/refresh".into(), 30, 60),
            ],
            login_per_ip_per_minute: config.rate_limit.login_per_ip_per_minute,
            api_per_principal_per_second: config.rate_limit.api_per_principal_per_second,
            ..Default::default()
        },
    );

    // Single router: optional JWT + API key auth on all /v1 routes.
    // Login/refresh don't extract AuthPrincipal so they pass through fine.
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/v1", api::routes::router(app_state)
            .layer(platform_middleware::auth::ApiKeyAuthLayer::new(api_key_lookup))
            .layer(platform_middleware::JwtAuthLayer::optional(config.auth.clone())))
        .layer(platform_middleware::SecurityHeadersLayer)
        .layer(platform_middleware::RequestIdLayer)
        .layer(rate_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("IAM service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let shutdown_signal = platform_middleware::shutdown_signal(
        platform_middleware::ShutdownConfig::critical("iam-service"),
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
        "checks": { "postgres": "not_checked", "redis": "not_checked" }
    }))
}

use axum::{routing::get, Router};
use platform_config::AppConfig;
use platform_logging::ServiceLogger;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod api;
mod application;
mod domain;
mod infrastructure;

use crate::application::services::OperatorServiceImpl;
use crate::infrastructure::adapters::PostgresOperatorRepository;
use crate::infrastructure::messaging::EventPublisher;

#[tokio::main]
async fn main() {
    ServiceLogger::init("operator-service");

    let config = AppConfig::from_env_or_panic("operator-service");

    // Connect to Postgres with retry
    let db = infrastructure::database::connect(&config.database).await;

    // Connect to Redis with retry
    let redis = infrastructure::cache::connect(&config.redis).await;

    // Connect to NATS JetStream
    let event_publisher = EventPublisher::new(&config.nats.url, "operator-events")
        .await
        .expect("Failed to connect to NATS");

    // Create repository
    let repo = PostgresOperatorRepository::new(db.clone());

    // Create service
    let service = OperatorServiceImpl::new(Box::new(repo), event_publisher, db.clone());

    // Create app state
    let app_state = api::AppState::new(db, service);

    // Build shared middleware stack
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

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("Operator service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let shutdown_signal = platform_middleware::shutdown_signal(
        platform_middleware::ShutdownConfig::critical("operator-service"),
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
    // TODO: inject db/redis pool via AppState for real checks
    // For now, return structured JSON per SRS HEALTH-005
    axum::Json(serde_json::json!({
        "status": "ok",
        "checks": {
            "postgres": "not_checked",
            "redis": "not_checked"
        }
    }))
}

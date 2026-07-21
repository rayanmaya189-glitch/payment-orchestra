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

use crate::application::services::ReconciliationServiceImpl;
use crate::infrastructure::adapters::PostgresSettlementRepository;

#[tokio::main]
async fn main() {
    platform_logging::install_panic_hook();
    ServiceLogger::init("reconciliation-service");
    let config = AppConfig::from_env_or_panic("reconciliation-service");
    let db = infrastructure::database::connect(&config.database).await;
    let redis = infrastructure::cache::connect(&config.redis).await;

    let batch_repo = PostgresSettlementRepository::new(db.clone());
    // For now, use the batch_repo as a stub for record/ledger/idempotency repos.
    // In production, separate repository implementations would be injected.
    let service = ReconciliationServiceImpl::new(
        Box::new(batch_repo),
        Box::new(infrastructure::adapters::InMemoryRecordRepository::new()),
        Box::new(infrastructure::adapters::InMemoryLedgerRepository::new()),
        Box::new(infrastructure::adapters::InMemoryIdempotencyGuard::new()),
        db.clone(),
    );

    let app_state = api::AppState::new(service);

    let cors = platform_middleware::cors_layer(&config.cors);
    let rate_limit = platform_middleware::RateLimitLayer::new(
        redis,
        platform_middleware::RateLimitLayerConfig::default(),
    );

    let app = Router::new()
        .route("/healthz", get(healthz))
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

    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    tracing::info!("Reconciliation service listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let shutdown = platform_middleware::shutdown_signal(
        platform_middleware::ShutdownConfig::standard("reconciliation-service"),
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .unwrap();
}

async fn healthz() -> &'static str {
    "ok"
}

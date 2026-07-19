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

use crate::application::services::ComplianceServiceImpl;
use crate::infrastructure::adapters::PostgresKybCaseRepository;

#[tokio::main]
async fn main() {
    ServiceLogger::init("compliance-service");

    let config = AppConfig::from_env("compliance-service").unwrap_or_default();

    // Connect to Postgres
    let db = infrastructure::database::connect(&config.database).await;

    // Create repository
    let repo = PostgresKybCaseRepository::new(db.clone());

    // Create service
    let service = ComplianceServiceImpl::new(Box::new(repo), db.clone());

    // Create app state
    let app_state = api::AppState {
        service: Arc::new(service),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(healthz))
        .nest("/v1", api::routes::router(app_state))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("Compliance service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn healthz() -> &'static str {
    "ok"
}

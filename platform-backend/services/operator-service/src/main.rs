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

    let config = AppConfig::from_env("operator-service").unwrap_or_default();

    // Connect to Postgres
    let db = infrastructure::database::connect(&config.database).await;

    // Connect to NATS JetStream
    let event_publisher = EventPublisher::new(&config.nats.url, "operator-events")
        .await
        .expect("Failed to connect to NATS");

    // Create repository
    let repo = PostgresOperatorRepository::new(db.clone());

    // Create service
    let service = OperatorServiceImpl::new(
        Box::new(repo),
        event_publisher,
        db.clone(),
    );

    // Create app state
    let app_state = api::AppState::new(db, service);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/v1", api::routes::router(app_state))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("Operator service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz() -> &'static str {
    // TODO: check Postgres ping, Redis PING
    "ok"
}

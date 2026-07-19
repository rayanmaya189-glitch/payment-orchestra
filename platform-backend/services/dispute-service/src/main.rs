use axum::{routing::get, Router}; use platform_config::AppConfig; use platform_logging::ServiceLogger;
use std::net::SocketAddr; use std::sync::Arc; use tower_http::trace::TraceLayer;
mod api; mod application; mod domain; mod infrastructure;
use crate::application::services::DisputeServiceImpl;

#[tokio::main]
async fn main() {
    ServiceLogger::init("dispute-service");
    let config = AppConfig::from_env("dispute-service").unwrap_or_default();
    let db = infrastructure::database::connect(&config.database).await;
    let service = DisputeServiceImpl::new(db.clone());
    let app_state = api::AppState { service: Arc::new(service) };
    let app = Router::new().route("/healthz", get(healthz)).route("/readyz", get(healthz)).nest("/v1", api::routes::router(app_state)).layer(TraceLayer::new_for_http());
    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    tracing::info!("Dispute service listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
async fn healthz() -> &'static str { "ok" }

use axum::{routing::get, Router};
use platform_config::AppConfig;
use platform_logging::ServiceLogger;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use platform_api::utoipa::OpenApi;

mod api;
mod application;
mod domain;
mod infrastructure;

use platform_api::ApiDoc;

#[tokio::main]
async fn main() {
    ServiceLogger::init("api-gateway");

    let config = AppConfig::from_env("api-gateway")
        .expect("Failed to load config from environment");

    let cors = platform_middleware::cors_layer(&config.cors);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        // Swagger UI at /docs
        .merge(utoipa_swagger_ui::SwaggerUi::new("/docs")
            .url("/api-docs/openapi.json", ApiDoc::openapi()))
        // OpenAPI JSON at /api-docs/openapi.json
        .route("/api-docs/openapi.json", get(openapi_json))
        .layer(platform_middleware::SecurityHeadersLayer)
        .layer(platform_middleware::ApiVersionLayer)
        .layer(platform_middleware::RequestIdLayer)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(
        config.server.host.parse().unwrap(),
        config.server.port,
    );

    tracing::info!("API Gateway listening on {addr}");
    tracing::info!("Swagger UI available at http://{addr}/docs");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let shutdown_signal = platform_middleware::shutdown_signal(
        platform_middleware::ShutdownConfig::critical("api-gateway"),
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
        "checks": {
            "postgres": "not_checked",
            "redis": "not_checked"
        }
    }))
}

async fn openapi_json() -> axum::Json<serde_json::Value> {
    let spec = ApiDoc::openapi();
    axum::Json(serde_json::to_value(spec).unwrap())
}

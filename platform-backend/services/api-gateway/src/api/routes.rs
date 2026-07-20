use axum::{extract::State, routing::{get, post}, Json, Router};
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::ApiGatewayService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/routes", get(list_routes))
        .route("/routes/resolve", post(resolve_route))
        .with_state(state)
}

async fn list_routes(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match state.service.list_routes().await {
        Ok(routes) => Ok(Json(serde_json::json!({"data": routes.iter().map(|r| serde_json::json!({"prefix": r.path_prefix, "service": r.target_service})).collect::<Vec<_>>()}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn resolve_route(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = RouteRequest { method: req["method"].as_str().unwrap_or("GET").to_string(), path: req["path"].as_str().unwrap_or("/").to_string() };
    match state.service.resolve_route(cmd).await {
        Ok(r) => Ok(Json(serde_json::json!({"target_service": r.target_service, "target_url": r.target_url, "auth_required": r.auth_required}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

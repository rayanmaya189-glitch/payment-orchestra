use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::ReconciliationService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/settlements/poll", post(poll_settlement))
        .route("/settlements/{batch_id}", get(get_batch))
        .with_state(state)
}

async fn poll_settlement(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = PollSettlementCommand {
        operator_id: req["operator_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        connector_id: req["connector_id"].as_str().unwrap_or("ni").to_string(),
        period_start: req["period_start"].as_str().unwrap_or("").to_string(),
        period_end: req["period_end"].as_str().unwrap_or("").to_string(),
    };
    match state.service.poll(cmd).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"batch_id": id.to_string()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_batch(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let bid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get_batch(bid).await {
        Ok(b) => Ok(Json(serde_json::json!({"batch_id": b.batch_id.to_string(), "status": b.status.as_str(), "total_records": b.total_records}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

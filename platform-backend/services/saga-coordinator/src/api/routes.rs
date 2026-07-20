use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::SagaService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/sagas", post(start_saga))
        .route("/sagas/{saga_id}", get(get_saga))
        .route("/sagas/{saga_id}/advance", post(advance_saga))
        .route("/sagas/{saga_id}/fail", post(fail_saga))
        .with_state(state)
}

async fn start_saga(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let steps: Vec<SagaStepDef> = req["steps"].as_array().map(|arr| {
        arr.iter().enumerate().map(|(i, s)| SagaStepDef {
            name: s["name"].as_str().unwrap_or("").to_string(),
            service: s["service"].as_str().unwrap_or("").to_string(),
            action: s["action"].as_str().unwrap_or("").to_string(),
            compensation_action: s["compensation_action"].as_str().map(String::from),
        }).collect()
    }).unwrap_or_default();
    let cmd = StartSagaCommand { saga_type: req["saga_type"].as_str().unwrap_or("payment").to_string(), steps, payload: req["payload"].clone() };
    match state.service.start(cmd).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"saga_id": id.to_string()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_saga(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get(sid).await {
        Ok(s) => Ok(Json(serde_json::json!({"saga_id": s.saga_id.to_string(), "status": s.status.as_str(), "current_step": s.current_step, "total_steps": s.total_steps}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn advance_saga(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.advance(AdvanceSagaCommand { saga_id: sid }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "advanced"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn fail_saga(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.fail(FailSagaCommand { saga_id: sid, error: req["error"].as_str().unwrap_or("unknown").to_string() }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "compensated"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

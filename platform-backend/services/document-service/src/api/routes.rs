use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::DocumentService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/documents/{document_id}", get(get_document))
        .route("/documents/{document_id}/verify", post(verify_document))
        .with_state(state)
}

async fn get_document(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get(did).await {
        Ok(d) => Ok(Json(serde_json::json!({"document_id": d.document_id.to_string(), "type": d.document_type, "status": d.status.as_str(), "verification": d.verification_status.as_str()}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn verify_document(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.verify(VerifyDocumentCommand { document_id: did, notes: req["notes"].as_str().unwrap_or("").to_string(), approved: req["approved"].as_bool().unwrap_or(true) }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "verified"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

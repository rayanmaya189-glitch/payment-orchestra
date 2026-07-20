use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::DisputeService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/disputes", post(open_dispute))
        .route("/disputes/{dispute_id}", get(get_dispute))
        .route("/disputes/{dispute_id}/evidence", post(submit_evidence))
        .route("/disputes/{dispute_id}/resolve", post(resolve_dispute))
        .with_state(state)
}

async fn open_dispute(State(state): State<AppState>, auth: AuthPrincipal, Json(req): Json<serde_json::Value>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = OpenDisputeCommand {
        payment_intent_id: req["payment_intent_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        operator_id: req["operator_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        reason: req["reason"].as_str().unwrap_or("unknown").to_string(),
        amount_minor_units: req["amount_minor_units"].as_i64().unwrap_or(0),
        currency: req["currency"].as_str().unwrap_or("AED").to_string(),
        acquirer_reference: req["acquirer_reference"].as_str().unwrap_or("").to_string(),
        connector_id: req["connector_id"].as_str().unwrap_or("").to_string(),
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };
    match state.service.open(cmd).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"dispute_id": id.to_string()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_dispute(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get(did).await {
        Ok(d) => Ok(Json(serde_json::json!({"dispute_id": d.dispute_id.to_string(), "status": d.status.as_str(), "reason": d.reason}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn submit_evidence(State(state): State<AppState>, auth: AuthPrincipal, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.submit_evidence(SubmitEvidenceCommand { dispute_id: did, evidence: req, principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "evidence_submitted"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn resolve_dispute(State(state): State<AppState>, auth: AuthPrincipal, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.resolve(ResolveDisputeCommand { dispute_id: did, decision: req["decision"].as_str().unwrap_or("won").to_string(), reason: req["reason"].as_str().unwrap_or("").to_string(), principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "resolved"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

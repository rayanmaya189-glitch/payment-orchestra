use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::RiskService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/risk/assess", post(assess_risk))
        .route("/risk/assessments/{id}", get(get_assessment))
        .with_state(state)
}

async fn assess_risk(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = AssessPaymentRiskCommand {
        payment_intent_id: req["payment_intent_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        operator_id: req["operator_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        amount_minor_units: req["amount_minor_units"].as_i64().unwrap_or(0),
        currency: req["currency"].as_str().unwrap_or("AED").to_string(),
        ip_address: req["ip_address"].as_str().map(String::from),
        user_agent: req["user_agent"].as_str().map(String::from),
    };
    match state.service.assess(cmd).await {
        Ok(a) => Ok(Json(serde_json::json!({"assessment_id": a.assessment_id.to_string(), "score": a.score, "decision": a.decision.as_str()}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_assessment(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let aid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get_assessment(aid).await {
        Ok(a) => Ok(Json(serde_json::json!({"assessment_id": a.assessment_id.to_string(), "score": a.score, "decision": a.decision.as_str()}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

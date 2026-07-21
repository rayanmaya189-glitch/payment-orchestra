use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::Serialize;
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::ComplianceService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/kyb-cases", post(submit_kyb).get(list_pending))
        .route("/kyb-cases/{case_id}", get(get_case))
        .route("/kyb-cases/{case_id}/assign", post(assign_officer))
        .route("/kyb-cases/{case_id}/decide", post(decide_kyb))
        .with_state(state)
}

#[derive(Serialize)]
struct KybCaseResponse { kyb_case_id: String, status: String, assigned_officer: Option<String>, risk_score: Option<f64> }

async fn submit_kyb(State(state): State<AppState>, auth: AuthPrincipal, Json(req): Json<serde_json::Value>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let requested_operator_id = req["operator_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
    let operator_id = shared_types::derive_operator_id(&auth.principal_id, &auth.role, requested_operator_id);
    match state.service.submit_kyb(SubmitKybCommand { operator_id, principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"kyb_case_id": id.to_string(), "status": "submitted"})))),
        Err(e) => {
            let msg = match &e {
                platform_error::PlatformError::Internal(m) => {
                    tracing::error!(error = %platform_logging::sanitize_error_message(m), "Internal error in compliance-service submit_kyb");
                    "Internal error".to_string()
                }
                _ => e.to_string(),
            };
            Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg}))))
        }
    }
}

async fn get_case(State(state): State<AppState>, auth: AuthPrincipal, Path(id): Path<String>) -> Result<Json<KybCaseResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let case_id = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get_case(GetKybCaseCommand { case_id, principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(c) => Ok(Json(KybCaseResponse { kyb_case_id: c.case_id.to_string(), status: c.status.as_str().to_string(), assigned_officer: c.assigned_officer.map(|u| u.to_string()), risk_score: c.risk_score })),
        Err(e) => {
            let msg = match &e {
                platform_error::PlatformError::Internal(m) => {
                    tracing::error!(error = %platform_logging::sanitize_error_message(m), "Internal error in compliance-service get_case");
                    "Internal error".to_string()
                }
                _ => e.to_string(),
            };
            Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": msg}))))
        }
    }
}

async fn assign_officer(State(state): State<AppState>, auth: AuthPrincipal, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let case_id = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    let officer_id = req["officer_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil());
    match state.service.assign_officer(AssignOfficerCommand { case_id, officer_id, principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "assigned"}))),
        Err(e) => {
            let msg = match &e {
                platform_error::PlatformError::Internal(m) => {
                    tracing::error!(error = %platform_logging::sanitize_error_message(m), "Internal error in compliance-service assign_officer");
                    "Internal error".to_string()
                }
                _ => e.to_string(),
            };
            Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg}))))
        }
    }
}

async fn decide_kyb(State(state): State<AppState>, auth: AuthPrincipal, Path(id): Path<String>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let case_id = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    let decision = req["decision"].as_str().unwrap_or("approved").to_string();
    let reason = req["reason"].as_str().unwrap_or("").to_string();
    match state.service.decide(DecideKybCommand { case_id, decision, reason, principal_id: auth.principal_id, role: auth.role.clone() }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "decided"}))),
        Err(e) => {
            let msg = match &e {
                platform_error::PlatformError::Internal(m) => {
                    tracing::error!(error = %platform_logging::sanitize_error_message(m), "Internal error in compliance-service decide_kyb");
                    "Internal error".to_string()
                }
                _ => e.to_string(),
            };
            Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg}))))
        }
    }
}

async fn list_pending() -> Json<serde_json::Value> { Json(serde_json::json!({"data": [], "has_more": false})) }

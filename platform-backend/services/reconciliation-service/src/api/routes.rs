use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::ReconciliationService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/settlements/poll", post(poll_settlement))
        .route("/settlements/ingest", post(ingest_settlement))
        .route("/settlements/{batch_id}", get(get_batch))
        .route("/settlements/{batch_id}/match", post(match_batch))
        .route("/settlements/{batch_id}/finalize", post(finalize_batch))
        .with_state(state)
}

async fn poll_settlement(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !matches!(auth.role.as_str(), "platform_admin" | "operator_admin" | "finance_officer") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let operator_id = req["operator_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());

    let cmd = PollSettlementCommand {
        operator_id,
        connector_id: req["connector_id"].as_str().unwrap_or("ni").to_string(),
        period_start: req["period_start"].as_str().unwrap_or("").to_string(),
        period_end: req["period_end"].as_str().unwrap_or("").to_string(),
    };

    match state.service.poll(cmd).await {
        Ok(id) => Ok((
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({"batch_id": id.to_string()})),
        )),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn ingest_settlement(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !matches!(auth.role.as_str(), "platform_admin" | "operator_admin" | "finance_officer") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let raw_content = req["raw_content"].as_str().unwrap_or("");
    if raw_content.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "raw_content is required", "code": "VALIDATION_ERROR"}))));
    }

    let operator_id = req["operator_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());

    let cmd = IngestSettlementCommand {
        operator_id,
        connector_id: req["connector_id"].as_str().unwrap_or("ni").to_string(),
        raw_content: raw_content.to_string(),
        file_format: req["file_format"].as_str().unwrap_or("csv").to_string(),
        file_checksum: req["file_checksum"].as_str().unwrap_or("").to_string(),
    };

    match state.service.ingest(cmd).await {
        Ok(id) => Ok((
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({"batch_id": id.to_string()})),
        )),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn get_batch(
    State(state): State<AppState>,
    _auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let bid = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid batch ID", "code": "INVALID_ID"})),
    ))?;

    match state.service.get_batch(bid).await {
        Ok(b) => Ok(Json(serde_json::json!({
            "batch_id": b.batch_id.to_string(),
            "status": b.status.as_str(),
            "total_records": b.total_records,
            "matched_count": b.matched_count,
            "unmatched_count": b.unmatched_count,
            "exception_count": b.exception_count,
        }))),
        Err(e) => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn match_batch(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !matches!(auth.role.as_str(), "platform_admin" | "operator_admin" | "finance_officer") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let bid = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid batch ID", "code": "INVALID_ID"})),
    ))?;

    let cmd = MatchSettlementCommand {
        batch_id: bid,
        matched_count: 0,
        unmatched_count: 0,
        exceptions: None,
    };

    match state.service.match_batch(cmd).await {
        Ok(result) => Ok((
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "batch_id": result.batch_id.to_string(),
                "matched": result.matched,
                "unmatched": result.unmatched,
                "exceptions": result.exceptions,
            })),
        )),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn finalize_batch(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only administrators can finalize batches", "code": "FORBIDDEN"}))));
    }

    let bid = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid batch ID", "code": "INVALID_ID"})),
    ))?;

    match state.service.finalize_batch(bid).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "settled"}))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

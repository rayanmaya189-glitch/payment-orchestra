use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::DisputeService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/disputes", post(open_dispute).get(list_disputes))
        .route("/disputes/{dispute_id}", get(get_dispute))
        .route("/disputes/{dispute_id}/evidence", post(submit_evidence))
        .route("/disputes/{dispute_id}/resolve", post(resolve_dispute))
        .with_state(state)
}

fn error_response(e: PlatformError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    match &e {
        PlatformError::Validation(ve) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": ve.to_string(),
                "code": "VALIDATION_ERROR",
            })),
        ),
        PlatformError::NotFound { resource, id } => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("{resource} {id} not found"),
                "code": "NOT_FOUND",
            })),
        ),
        PlatformError::Conflict(_) => (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": e.to_string(),
                "code": "CONFLICT",
            })),
        ),
        PlatformError::AuthorizationDenied(_) => (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": e.to_string(),
                "code": "FORBIDDEN",
            })),
        ),
        _ => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Internal server error",
                "code": "INTERNAL_ERROR",
            })),
        ),
    }
}

async fn open_dispute(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let payment_intent_id = req["payment_intent_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid payment_intent_id", "code": "VALIDATION_ERROR"})),
            )
        })?;
    let operator_id = req["operator_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid operator_id", "code": "VALIDATION_ERROR"})),
            )
        })?;

    let cmd = OpenDisputeCommand {
        payment_intent_id,
        operator_id,
        reason: req["reason"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        amount_minor_units: req["amount_minor_units"].as_i64().unwrap_or(0),
        currency: req["currency"].as_str().unwrap_or("AED").to_string(),
        acquirer_reference: req["acquirer_reference"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        connector_id: req["connector_id"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.open(cmd).await {
        Ok(id) => Ok((
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({"dispute_id": id.to_string()})),
        )),
        Err(e) => Err(error_response(e)),
    }
}

async fn get_dispute(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !matches!(auth.role.as_str(), "platform_admin" | "compliance_officer" | "operator_admin") {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"})),
        ));
    }

    let did = Uuid::parse_str(&id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid dispute ID", "code": "INVALID_ID"})),
        )
    })?;

    match state.service.get(did).await {
        Ok(d) => Ok(Json(serde_json::json!({
            "dispute_id": d.dispute_id.to_string(),
            "status": d.status.as_str(),
            "reason": d.reason,
            "operator_id": d.operator_id.to_string(),
            "payment_intent_id": d.payment_intent_id.to_string(),
            "disputed_amount": {
                "amount_minor_units": d.disputed_amount.amount_minor_units,
                "currency": d.disputed_amount.currency.0,
            },
            "decision": d.decision.as_ref().map(|dec| dec.as_str()),
            "decision_reason": d.decision_reason,
            "opened_at": d.opened_at.to_rfc3339(),
            "resolved_at": d.resolved_at.map(|dt| dt.to_rfc3339()),
        }))),
        Err(e) => Err(error_response(e)),
    }
}

async fn list_disputes(
    State(state): State<AppState>,
    auth: AuthPrincipal,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if !matches!(auth.role.as_str(), "platform_admin" | "compliance_officer") {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"})),
        ));
    }

    // Platform admins see all disputes; compliance officers and operators see scoped
    // For simplicity, accept optional operator_id query param — if not present, return all
    // In production this would be a query parameter extractor
    let disputes = match state.service.list(auth.principal_id).await {
        Ok(d) => d,
        Err(e) => return Err(error_response(e)),
    };

    let items: Vec<serde_json::Value> = disputes
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "dispute_id": d.dispute_id.to_string(),
                "status": d.status.as_str(),
                "reason": d.reason,
                "operator_id": d.operator_id.to_string(),
                "payment_intent_id": d.payment_intent_id.to_string(),
                "disputed_amount": {
                    "amount_minor_units": d.disputed_amount.amount_minor_units,
                    "currency": d.disputed_amount.currency.0,
                },
                "decision": d.decision.as_ref().map(|dec| dec.as_str()),
                "opened_at": d.opened_at.to_rfc3339(),
                "resolved_at": d.resolved_at.map(|dt| dt.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "disputes": items,
        "count": items.len(),
    })))
}

async fn submit_evidence(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid dispute ID", "code": "INVALID_ID"})),
        )
    })?;

    let operator_id = req["operator_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid operator_id", "code": "VALIDATION_ERROR"})),
            )
        })?;

    match state
        .service
        .submit_evidence(SubmitEvidenceCommand {
            dispute_id: did,
            evidence: req["evidence"].clone(),
            principal_id: auth.principal_id,
            role: auth.role.clone(),
            operator_id,
        })
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({"status": "evidence_submitted"}))),
        Err(e) => Err(error_response(e)),
    }
}

async fn resolve_dispute(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let did = Uuid::parse_str(&id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid dispute ID", "code": "INVALID_ID"})),
        )
    })?;

    match state
        .service
        .resolve(ResolveDisputeCommand {
            dispute_id: did,
            decision: req["decision"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            reason: req["reason"].as_str().unwrap_or("").to_string(),
            principal_id: auth.principal_id,
            role: auth.role.clone(),
        })
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({"status": "resolved"}))),
        Err(e) => Err(error_response(e)),
    }
}

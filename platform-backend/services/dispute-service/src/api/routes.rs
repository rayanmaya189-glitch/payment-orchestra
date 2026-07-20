use axum::{extract::{Path, State}, http::StatusCode, Json, Router};
use uuid::Uuid;
use super::dto::*;
use super::AppState;
use crate::application::services::{DisputeService, DisputeResponse as ServiceResponse};
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/disputes", axum::routing::post(open_dispute))
        .route("/disputes/{id}", axum::routing::get(get_dispute))
        .route("/disputes/{id}/evidence", axum::routing::post(submit_evidence))
        .route("/disputes/{id}/resolve", axum::routing::post(resolve_dispute))
        .with_state(state)
}

async fn open_dispute(State(state): State<AppState>, _auth: AuthPrincipal, Json(req): Json<OpenDisputeRequest>) -> Result<(StatusCode, Json<DisputeResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::OpenDisputeCommand {
        operator_id: Uuid::nil(),
        payment_intent_id: req.payment_intent_id,
        reason: req.reason,
        amount: req.amount,
    };
    match state.service.open_dispute(cmd).await {
        Ok(r) => Ok((StatusCode::CREATED, Json(convert_response(r)))),
        Err(e) => Err(err(e)),
    }
}

async fn get_dispute(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<DisputeResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_dispute(id).await {
        Ok(r) => Ok(Json(convert_response(r))),
        Err(e) => Err(err(e)),
    }
}

async fn submit_evidence(State(state): State<AppState>, Path(id): Path<Uuid>, Json(req): Json<SubmitEvidenceRequest>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.submit_evidence(id, req.evidence).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

async fn resolve_dispute(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.resolve_dispute(id).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

fn convert_response(r: ServiceResponse) -> DisputeResponse {
    DisputeResponse { dispute_id: r.dispute_id, status: r.status, reason: r.reason, amount: r.amount, currency: r.currency }
}

fn err(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (s, c, m) = match &e {
        platform_error::PlatformError::NotFound { resource, id } => (StatusCode::NOT_FOUND, "NOT_FOUND", format!("{resource} {id}")),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal error".into()),
    };
    (s, Json(ErrorResponse { error: m, code: c.to_string() }))
}

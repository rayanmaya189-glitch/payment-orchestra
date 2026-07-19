use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::{ErrorResponse, OperatorResponse, RegisterOperatorRequest, UpdateOperatorStatusRequest};
use super::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/operators", axum::routing::post(register_operator))
        .route("/operators/{operator_id}", axum::routing::get(get_operator))
        .route("/operators/{operator_id}/verify-email", axum::routing::post(verify_email))
        .route("/operators/{operator_id}/status", axum::routing::put(update_status))
        .with_state(state)
}

async fn register_operator(
    State(_state): State<AppState>,
    Json(req): Json<RegisterOperatorRequest>,
) -> Result<(StatusCode, Json<OperatorResponse>), (StatusCode, Json<ErrorResponse>)> {
    let response = OperatorResponse {
        id: Uuid::now_v7(),
        legal_name: req.legal_name,
        trade_license_no: req.trade_license_no,
        country: req.country,
        status: "pending".to_string(),
        subdomain: String::new(),
        email: req.email,
        provisioned_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

async fn get_operator(
    State(_state): State<AppState>,
    Path(_operator_id): Path<Uuid>,
) -> Result<Json<OperatorResponse>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "Not found".to_string(),
            code: "OPERATOR_NOT_FOUND".to_string(),
        }),
    ))
}

async fn verify_email(
    State(_state): State<AppState>,
    Path(_operator_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::OK)
}

async fn update_status(
    State(_state): State<AppState>,
    Path(_operator_id): Path<Uuid>,
    Json(_req): Json<UpdateOperatorStatusRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::OK)
}

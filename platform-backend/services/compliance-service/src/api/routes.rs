use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/kyb-cases", axum::routing::post(create_kyb_case))
        .route("/kyb-cases", axum::routing::get(list_kyb_cases))
        .route("/kyb-cases/{case_id}", axum::routing::get(get_kyb_case))
        .route("/kyb-cases/{case_id}/documents", axum::routing::post(upload_document))
        .route("/kyb-cases/{case_id}/decide", axum::routing::post(decide_case))
        .route("/kyb-cases/{case_id}/assign", axum::routing::post(assign_officer))
        .with_state(state)
}

async fn create_kyb_case(
    State(state): State<AppState>,
    Json(req): Json<CreateKybCaseRequest>,
) -> Result<(StatusCode, Json<KybCaseResponse>), (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, req);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn list_kyb_cases(
    State(state): State<AppState>,
) -> Result<Json<Vec<KybCaseResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let _ = &state;
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn get_kyb_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> Result<Json<KybCaseResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, case_id);
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "Not found".to_string(),
            code: "KYB_CASE_NOT_FOUND".to_string(),
        }),
    ))
}

async fn upload_document(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(req): Json<UploadDocumentRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, case_id, req);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn decide_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(req): Json<DecideCaseRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, case_id, req);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn assign_officer(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, case_id);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

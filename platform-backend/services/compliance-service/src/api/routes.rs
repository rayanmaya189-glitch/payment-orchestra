use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::commands::*;
use crate::application::queries::*;
use crate::application::services::ComplianceService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/kyb-cases", axum::routing::post(create_kyb_case))
        .route("/kyb-cases", axum::routing::get(list_kyb_cases))
        .route("/kyb-cases/{case_id}", axum::routing::get(get_kyb_case))
        .route("/kyb-cases/{case_id}/documents", axum::routing::post(upload_document))
        .route("/kyb-cases/{case_id}/decide", axum::routing::post(decide_case))
        .route("/kyb-cases/{case_id}/assign", axum::routing::post(assign_officer))
        .route("/kyb-cases/{case_id}/request-documents", axum::routing::post(request_documents))
        .with_state(state)
}

async fn create_kyb_case(
    State(state): State<AppState>,
    Json(req): Json<CreateKybCaseRequest>,
) -> Result<(StatusCode, Json<KybCaseResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = CreateKybCaseCommand {
        operator_id: req.operator_id,
    };

    match state.service.create_kyb_case(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_kyb_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> Result<Json<KybCaseResponse>, (StatusCode, Json<ErrorResponse>)> {
    let query = GetKybCaseQuery { kyb_case_id: case_id };

    match state.service.get_case(query).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn list_kyb_cases(
    State(state): State<AppState>,
) -> Result<Json<Vec<KybCaseResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let query = ListKybCasesQuery {
        status: None,
        operator_id: None,
        cursor: None,
        limit: Some(20),
    };

    match state.service.list_cases(query).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn upload_document(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(req): Json<UploadDocumentRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = UploadDocumentCommand {
        kyb_case_id: case_id,
        document_type: req.document_type,
        file_key: req.file_key,
        file_hash: req.file_hash,
    };

    match state.service.upload_document(cmd).await {
        Ok(()) => Ok(StatusCode::CREATED),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn decide_case(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(case_id): Path<Uuid>,
    Json(req): Json<DecideCaseRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = DecideKybCaseCommand {
        kyb_case_id: case_id,
        decision: req.decision,
        reason: req.reason,
        decided_by: auth.principal_id,
    };

    match state.service.decide_case(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn assign_officer(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(req): Json<AssignOfficerRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = AssignOfficerCommand {
        kyb_case_id: case_id,
        officer_id: req.officer_id,
    };

    match state.service.assign_officer(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn request_documents(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(req): Json<RequestDocumentsRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = RequestDocumentsCommand {
        kyb_case_id: case_id,
        requested_documents: req.requested_documents,
        reason: req.reason,
    };

    match state.service.request_documents(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

fn error_to_response(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code, message) = match &e {
        platform_error::PlatformError::NotFound { resource, id } => (
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("{resource} {id} not found"),
        ),
        platform_error::PlatformError::Conflict(c) => (
            StatusCode::CONFLICT,
            "CONFLICT",
            c.to_string(),
        ),
        platform_error::PlatformError::Validation(v) => (
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            v.to_string(),
        ),
        platform_error::PlatformError::AuthorizationDenied(msg) => (
            StatusCode::FORBIDDEN,
            "AUTHORIZATION_DENIED",
            msg.clone(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal error".to_string(),
        ),
    };

    (status, Json(ErrorResponse { error: message, code: code.to_string() }))
}

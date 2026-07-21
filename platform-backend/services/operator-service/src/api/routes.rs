use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::{ErrorResponse, RegisterOperatorRequest, UpdateOperatorStatusRequest};
use super::AppState;
use crate::application::commands::*;
use crate::application::queries::*;
use crate::application::services::OperatorService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/operators", axum::routing::post(register_operator))
        .route("/operators", axum::routing::get(list_operators))
        .route("/operators/{operator_id}", axum::routing::get(get_operator))
        .route("/operators/{operator_id}/verify-email", axum::routing::post(verify_email))
        .route("/operators/{operator_id}/status", axum::routing::put(update_status))
        .with_state(state)
}

async fn register_operator(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<RegisterOperatorRequest>,
) -> Result<(StatusCode, Json<crate::api::dto::OperatorResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = RegisterOperatorCommand {
        principal_id: auth.principal_id,
        role: auth.role,
        legal_name: req.legal_name,
        trade_license_no: req.trade_license_no,
        country: req.country,
        email: req.email,
    };

    match state.service.register_operator(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_operator(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<Uuid>,
) -> Result<Json<crate::api::dto::OperatorResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ABAC: read requires "read" permission on "operator"
    if let Err(e) = platform_middleware::evaluate_policy(&platform_middleware::AbacContext {
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        action: "read".to_string(),
        resource: "operator".to_string(),
        resource_id: Some(operator_id),
        amount: None,
        ip_address: None,
        operator_id: Some(operator_id),
    }) {
        return Err(error_to_response(e));
    }

    let query = GetOperatorQuery { operator_id };

    match state.service.get_operator(query).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn list_operators(
    State(state): State<AppState>,
    auth: AuthPrincipal,
) -> Result<Json<Vec<crate::api::dto::OperatorResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // ABAC: list requires "read" permission on "operator"
    if let Err(e) = platform_middleware::evaluate_policy(&platform_middleware::AbacContext {
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        action: "read".to_string(),
        resource: "operator".to_string(),
        resource_id: None,
        amount: None,
        ip_address: None,
        operator_id: None,
    }) {
        return Err(error_to_response(e));
    }

    let query = ListOperatorsQuery {
        status: None,
        cursor: None,
        limit: Some(20),
    };

    match state.service.list_operators(query).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn verify_email(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = VerifyEmailCommand {
        principal_id: auth.principal_id,
        role: auth.role,
        operator_id,
    };

    match state.service.verify_email(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn update_status(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<Uuid>,
    Json(req): Json<UpdateOperatorStatusRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = UpdateOperatorStatusCommand {
        principal_id: auth.principal_id,
        role: auth.role,
        operator_id,
        new_status: req.new_status,
        reason: req.reason,
    };

    match state.service.update_status(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

fn error_to_response(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code, message) = match &e {
        platform_error::PlatformError::NotFound { resource, id } => (
            StatusCode::NOT_FOUND,
            "OPERATOR_NOT_FOUND",
            format!("{resource} {id} not found"),
        ),
        platform_error::PlatformError::Conflict(c) => {
            let code = match c {
                platform_error::ConflictError::IdempotencyKeyConflict => "DUPLICATE_TRADE_LICENSE",
                platform_error::ConflictError::ConcurrencyViolation => "CONFLICT",
                _ => "CONFLICT",
            };
            (StatusCode::CONFLICT, code, c.to_string())
        }
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
        platform_error::PlatformError::RateLimited { retry_after_ms } => (
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            format!("Retry after {retry_after_ms}ms"),
        ),
        platform_error::PlatformError::Unavailable(msg) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            msg.clone(),
        ),
        platform_error::PlatformError::Internal(msg) => {
            tracing::error!(error = %platform_logging::sanitize_error_message(msg), "Internal error in operator-service");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Internal error".to_string(),
            )
        }
    };

    (status, Json(ErrorResponse { error: message, code: code.to_string() }))
}

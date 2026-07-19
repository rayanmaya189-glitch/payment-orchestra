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
use crate::application::services::AuthService;
use crate::domain::value_objects::PermissionContext;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", axum::routing::post(login))
        .route("/auth/refresh", axum::routing::post(refresh_token))
        .route(
            "/principals/{principal_id}/permissions",
            axum::routing::post(validate_permission),
        )
        .route(
            "/principals/{principal_id}/api-keys",
            axum::routing::post(create_api_key),
        )
        .route(
            "/api-keys/{api_key_id}",
            axum::routing::delete(revoke_api_key),
        )
        .with_state(state)
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = AuthenticateCommand {
        email: req.email,
        password: req.password,
        ip_address: "127.0.0.1".parse().unwrap(),
        user_agent: "unknown".into(),
    };

    match state.service.authenticate(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = IssueTokenCommand {
        refresh_token: req.refresh_token,
    };

    match state.service.issue_token(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn validate_permission(
    State(state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<ValidatePermissionRequest>,
) -> Result<Json<PermissionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let query = ValidatePermissionQuery {
        principal_id,
        resource: req.resource,
        action: req.action,
        context: PermissionContext {
            amount: req.context.as_ref().and_then(|c| c.amount),
            acquirer_link_id: req.context.as_ref().and_then(|c| c.acquirer_link_id),
        },
    };

    match state.service.validate_permission(query).await {
        Ok(result) => Ok(Json(PermissionResponse {
            allowed: result.allowed,
            requires_maker_checker: result.requires_maker_checker,
        })),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn create_api_key(
    State(state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = CreateApiKeyCommand {
        principal_id,
        name: req.name,
        scopes: req.scopes,
        acquirer_link_ids: req.acquirer_link_ids,
        expires_in_days: req.expires_in_days,
    };

    match state.service.create_api_key(cmd).await {
        Ok(result) => Ok((StatusCode::CREATED, Json(ApiKeyResponse {
            api_key_id: result.api_key_id,
            api_key_secret: result.api_key_secret,
        }))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Path(api_key_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = RevokeApiKeyCommand { api_key_id };

    match state.service.revoke_api_key(cmd).await {
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
        platform_error::PlatformError::AuthorizationDenied(msg) => (
            StatusCode::UNAUTHORIZED,
            "AUTHORIZATION_DENIED",
            msg.clone(),
        ),
        platform_error::PlatformError::Validation(v) => (
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            v.to_string(),
        ),
        platform_error::PlatformError::Conflict(c) => (
            StatusCode::CONFLICT,
            "CONFLICT",
            c.to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal error".to_string(),
        ),
    };

    (status, Json(ErrorResponse { error: message, code: code.to_string() }))
}

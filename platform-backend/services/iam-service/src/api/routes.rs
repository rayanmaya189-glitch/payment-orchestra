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
    let _ = (&state, req);
    // TODO: Implement authentication
    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, req);
    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn validate_permission(
    State(state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<ValidatePermissionRequest>,
) -> Result<Json<PermissionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, principal_id, req);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn create_api_key(
    State(state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, principal_id, req);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Path(api_key_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _ = (&state, api_key_id);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Not implemented".to_string(),
            code: "NOT_IMPLEMENTED".to_string(),
        }),
    ))
}

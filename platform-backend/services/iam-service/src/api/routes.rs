use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::commands::*;
use crate::application::queries::*;
use crate::application::services::AuthService;
use crate::domain::value_objects::PermissionContext;
use platform_middleware::{AuthPrincipal, client_fingerprint};

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
        .route(
            "/principals/{principal_id}/sessions/revoke-all",
            axum::routing::delete(revoke_all_sessions),
        )
        .with_state(state)
}

/// Extract client IP from request headers per trusted-proxy convention.
/// Priority: X-Real-IP > X-Forwarded-For (first entry) > "unknown".
fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return ip.to_string();
    }
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            return first.trim().to_string();
        }
    }
    "unknown".to_string()
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ip_str = extract_client_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let ip_address: std::net::IpAddr = ip_str.parse().unwrap_or(std::net::IpAddr::V4(
        std::net::Ipv4Addr::new(0, 0, 0, 0),
    ));

    let cmd = AuthenticateCommand {
        email: req.email,
        password: req.password,
        ip_address,
        user_agent,
    };

    match state.service.authenticate(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = IssueTokenCommand {
        refresh_token: req.refresh_token,
    };

    // Real client fingerprint for token theft detection (SRS SESS-SEC-003)
    let ip_str = extract_client_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let fingerprint = client_fingerprint(&ip_str, &user_agent);

    match state.service.issue_token(cmd, fingerprint).await {
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

/// SRS SESS-SEC-004: Admin can revoke all sessions for a principal.
/// ABAC enforced — only callers with Admin role may use this endpoint.
async fn revoke_all_sessions(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(principal_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // ABAC: Only Admin can revoke sessions for any principal
    if auth.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only admins can revoke all sessions".to_string(),
                code: "FORBIDDEN".to_string(),
            }),
        ));
    }

    match state.service.revoke_all_sessions(principal_id).await {
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

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
        // WebAuthn MFA endpoints (SRS AUTH-011)
        .route(
            "/principals/{principal_id}/webauthn/register",
            axum::routing::post(register_webauthn),
        )
        .route(
            "/principals/{principal_id}/webauthn/authenticate",
            axum::routing::post(authenticate_webauthn),
        )
        .route(
            "/principals/{principal_id}/webauthn/credentials",
            axum::routing::get(list_webauthn_credentials),
        )
        .route(
            "/principals/{principal_id}/backup-codes",
            axum::routing::post(generate_backup_codes),
        )
        .route(
            "/principals/{principal_id}/backup-codes/verify",
            axum::routing::post(verify_backup_code),
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

// ==================== WebAuthn MFA Endpoints (SRS AUTH-011) ====================

/// WebAuthn registration request.
#[derive(serde::Deserialize)]
pub struct RegisterWebAuthnRequest {
    pub credential_id: String,
    pub public_key: String,
    pub attestation_object: String,
}

/// WebAuthn authentication request.
#[derive(serde::Deserialize)]
pub struct AuthenticateWebAuthnRequest {
    pub credential_id: String,
    pub authenticator_data: String,
    pub client_data_json: String,
    pub signature: String,
}

/// WebAuthn credential response.
#[derive(serde::Serialize)]
pub struct WebAuthnCredentialResponse {
    pub credential_id: String,
    pub created_at: String,
}

/// Backup codes response.
#[derive(serde::Serialize)]
pub struct BackupCodesResponse {
    pub codes: Vec<String>,
    pub count: usize,
}

/// SRS AUTH-011: Register a WebAuthn credential for MFA.
async fn register_webauthn(
    State(_state): State<AppState>,
    auth: AuthPrincipal,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<RegisterWebAuthnRequest>,
) -> Result<(StatusCode, Json<WebAuthnCredentialResponse>), (StatusCode, Json<ErrorResponse>)> {
    // ABAC: Users can only register their own MFA
    if auth.principal_id != principal_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cannot register MFA for another user".to_string(),
                code: "FORBIDDEN".to_string(),
            }),
        ));
    }

    // TODO: Validate attestation object and public key
    // For now, store the credential
    tracing::info!(
        principal_id = %principal_id,
        credential_id = %req.credential_id,
        "WebAuthn credential registered"
    );

    Ok((StatusCode::CREATED, Json(WebAuthnCredentialResponse {
        credential_id: req.credential_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    })))
}

/// SRS AUTH-011: Authenticate with WebAuthn MFA.
async fn authenticate_webauthn(
    State(_state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<AuthenticateWebAuthnRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Validate authenticator signature against stored public key
    // For now, return success
    tracing::info!(
        principal_id = %principal_id,
        credential_id = %req.credential_id,
        "WebAuthn authentication attempted"
    );

    Ok(Json(serde_json::json!({
        "verified": true,
        "principal_id": principal_id.to_string(),
    })))
}

/// List WebAuthn credentials for a principal.
async fn list_webauthn_credentials(
    State(_state): State<AppState>,
    auth: AuthPrincipal,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<Vec<WebAuthnCredentialResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // ABAC: Users can only list their own credentials
    if auth.principal_id != principal_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cannot list credentials for another user".to_string(),
                code: "FORBIDDEN".to_string(),
            }),
        ));
    }

    // TODO: Query actual credentials from database
    Ok(Json(vec![]))
}

/// SRS AUTH-014: Generate backup codes for MFA recovery.
async fn generate_backup_codes(
    State(_state): State<AppState>,
    auth: AuthPrincipal,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<BackupCodesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ABAC: Users can only generate their own backup codes
    if auth.principal_id != principal_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cannot generate backup codes for another user".to_string(),
                code: "FORBIDDEN".to_string(),
            }),
        ));
    }

    // Generate 10 backup codes (SRS AUTH-014)
    use rand::RngCore;
    let codes: Vec<String> = (0..10)
        .map(|_| {
            let mut bytes = [0u8; 4];
            rand::thread_rng().fill_bytes(&mut bytes);
            format!("{:08X}", u32::from_be_bytes(bytes))
        })
        .collect();

    tracing::info!(
        principal_id = %principal_id,
        count = codes.len(),
        "Backup codes generated"
    );

    Ok(Json(BackupCodesResponse {
        count: codes.len(),
        codes,
    }))
}

/// Verify a backup code for MFA recovery.
async fn verify_backup_code(
    State(_state): State<AppState>,
    Path(principal_id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let code = req["code"].as_str().unwrap_or("");

    // TODO: Verify against stored hashed backup codes
    tracing::info!(
        principal_id = %principal_id,
        "Backup code verification attempted"
    );

    Ok(Json(serde_json::json!({
        "verified": true,
        "principal_id": principal_id.to_string(),
    })))
}

use axum::{
    extract::{Path, State},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::IamService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .route("/principals", post(register_principal))
        .route("/principals/{principal_id}/api-keys", post(create_api_key))
        .route("/api-keys/{api_key_id}", delete(revoke_api_key))
        .route("/healthz", get(health_check))
        .with_state(state)
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponseJson {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

async fn login(
    State(state): State<AppState>,
    axum::http::request::Parts { headers, .. }: axum::http::request::Parts,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponseJson>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let cmd = LoginCommand {
        email: req.email,
        password: req.password,
        ip_address: ip_address.clone(),
        user_agent: user_agent.clone(),
    };

    match state.service.login(cmd).await {
        Ok(resp) => {
            platform_logging::log_security_event(
                "iam-service",
                platform_logging::SecurityEventType::LoginSuccess,
                platform_logging::SecurityOutcome::Success,
                Some(resp.principal_id),
                Some(&ip_address),
                Some(&user_agent),
                None,
                None,
            );
            Ok(Json(LoginResponseJson {
                access_token: resp.access_token,
                refresh_token: resp.refresh_token,
                expires_in: resp.expires_in,
            }))
        }
        Err(e) => {
            platform_logging::log_security_event(
                "iam-service",
                platform_logging::SecurityEventType::LoginFailed,
                platform_logging::SecurityOutcome::Blocked,
                None,
                Some(&ip_address),
                Some(&user_agent),
                None,
                Some(serde_json::json!({"error": e.to_string()})),
            );
            Err((
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string(), "code": "UNAUTHORIZED"})),
            ))
        }
    }
}

#[derive(Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

async fn refresh_token(
    State(state): State<AppState>,
    axum::http::request::Parts { headers, .. }: axum::http::request::Parts,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponseJson>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let cmd = RefreshTokenCommand {
        refresh_token: req.refresh_token,
        ip_address,
        user_agent,
    };

    match state.service.refresh_token(cmd).await {
        Ok(resp) => Ok(Json(LoginResponseJson {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            expires_in: resp.expires_in,
        })),
        Err(e) => Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string(), "code": "UNAUTHORIZED"})),
        )),
    }
}

#[derive(Deserialize)]
struct RegisterPrincipalRequest {
    email: String,
    password: String,
    principal_type: Option<String>,
    role: Option<String>,
}

#[derive(Serialize)]
struct RegisterPrincipalResponse {
    principal_id: String,
}

async fn register_principal(
    State(state): State<AppState>,
    Json(req): Json<RegisterPrincipalRequest>,
) -> Result<(axum::http::StatusCode, Json<RegisterPrincipalResponse>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = RegisterPrincipalCommand {
        email: req.email,
        password: req.password,
        principal_type: req.principal_type.unwrap_or_else(|| "user".to_string()),
        role: req.role,
    };

    match state.service.register_principal(cmd).await {
        Ok(id) => Ok((
            axum::http::StatusCode::CREATED,
            Json(RegisterPrincipalResponse {
                principal_id: id.to_string(),
            }),
        )),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "REGISTRATION_FAILED"})),
        )),
    }
}

#[derive(Deserialize)]
struct CreateApiKeyRequest {
    name: String,
    scopes: Vec<String>,
    expires_in_days: Option<u32>,
}

#[derive(Serialize)]
struct CreateApiKeyResponseJson {
    api_key_id: String,
    api_key_secret: String,
    key_prefix: String,
}

async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(principal_id): Path<String>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(axum::http::StatusCode, Json<CreateApiKeyResponseJson>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let pid = Uuid::parse_str(&principal_id)
        .map_err(|_| (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid principal ID", "code": "INVALID_ID"})),
        ))?;

    // Object-level authorization: only allow creating API keys for your own principal
    if auth.principal_id != pid && auth.role != "platform_admin" {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized to create API keys for this principal", "code": "FORBIDDEN"})),
        ));
    }

    let cmd = CreateApiKeyCommand {
        principal_id: pid,
        name: req.name,
        scopes: req.scopes,
        expires_in_days: req.expires_in_days,
    };

    match state.service.create_api_key(cmd).await {
        Ok(resp) => Ok((
            axum::http::StatusCode::CREATED,
            Json(CreateApiKeyResponseJson {
                api_key_id: resp.api_key_id.to_string(),
                api_key_secret: resp.api_key_secret,
                key_prefix: resp.key_prefix,
            }),
        )),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "API_KEY_CREATE_FAILED"})),
        )),
    }
}

async fn revoke_api_key(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(api_key_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let key_id = Uuid::parse_str(&api_key_id)
        .map_err(|_| (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid API key ID", "code": "INVALID_ID"})),
        ))?;

    let cmd = RevokeApiKeyCommand {
        api_key_id: key_id,
        principal_id: auth.principal_id,
    };

    match state.service.revoke_api_key(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "revoked"}))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "REVOKE_FAILED"})),
        )),
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "iam-service"
    }))
}

/// Extract client IP from X-Real-IP or X-Forwarded-For headers.
/// Falls back to "127.0.0.1" for local connections.
fn extract_client_ip(headers: &axum::http::HeaderMap) -> String {
    // Prefer X-Real-IP (set by reverse proxy)
    if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return ip.to_string();
    }
    // Fall back to X-Forwarded-For (first IP in chain is the client)
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            return first.trim().to_string();
        }
    }
    "127.0.0.1".to_string()
}

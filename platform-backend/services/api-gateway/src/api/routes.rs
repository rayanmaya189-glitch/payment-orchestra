use base64::Engine;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::api::AppState;
use crate::api::dto::*;
use crate::application::commands::*;
use crate::application::services::ApiGatewayService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/routes", get(list_routes).post(create_route))
        .route("/routes/resolve", post(resolve_route))
        .route("/routes/forward", post(forward_request))
        .route("/routes/stats", get(get_stats))
        .route("/routes/{route_id}", get(get_route_health).put(update_route).delete(delete_route))
        .with_state(state)
}

fn platform_error_to_response(e: PlatformError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    match &e {
        PlatformError::Validation(_) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string(), "code": "VALIDATION_ERROR"})),
        ),
        PlatformError::NotFound { .. } => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string(), "code": "NOT_FOUND"})),
        ),
        PlatformError::AuthorizationDenied(_) => (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": e.to_string(), "code": "AUTHORIZATION_DENIED"})),
        ),
        PlatformError::RateLimited { retry_after_ms } => (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": e.to_string(),
                "code": "RATE_LIMITED",
                "retry_after_ms": retry_after_ms
            })),
        ),
        _ => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Internal error", "code": "INTERNAL_ERROR"})),
        ),
    }
}

async fn list_routes(
    State(state): State<AppState>,
    auth: AuthPrincipal,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can list routes
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    match state.service.list_routes().await {
        Ok(result) => Ok(Json(serde_json::json!({
            "data": result.routes.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn resolve_route(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<RouteRequestDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can resolve routes
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let cmd = RouteRequest {
        method: req.method,
        path: req.path,
    };

    match state.service.resolve_route(cmd).await {
        Ok(route) => Ok(Json(serde_json::json!({
            "target_service": route.target_service,
            "target_url": route.target_url,
            "auth_required": route.auth_required,
            "rate_limit": route.rate_limit.as_ref().map(|rl| serde_json::json!({
                "max_requests": rl.max_requests,
                "window_seconds": rl.window_seconds,
            })),
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn forward_request(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<ForwardRequestDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can forward requests
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let body_bytes = req
        .body
        .and_then(|b| base64::engine::general_purpose::STANDARD.decode(&b).ok())
        .unwrap_or_default();

    let cmd = ForwardRequestCommand {
        method: req.method,
        path: req.path,
        headers: req.headers.unwrap_or_default(),
        body: body_bytes,
        principal_id: Some(auth.principal_id.to_string()),
    };

    match state.service.forward_request(cmd).await {
        Ok(result) => match result.result {
            Some(r) => Ok(Json(serde_json::json!({
                "request_id": r.request_id.to_string(),
                "method": r.method,
                "path": r.path,
                "target_service": r.target_service,
                "status": r.status.label(),
                "response_status": r.response_status,
                "response_time_ms": r.response_time_ms,
                "error_message": r.error_message,
            }))),
            None => Ok(Json(serde_json::json!({"error": "No result"}))),
        },
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn create_route(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<CreateRouteDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can create routes
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let cmd = CreateRouteCommand {
        path_prefix: req.path_prefix,
        target_service: req.target_service,
        target_url: req.target_url,
        auth_required: req.auth_required.unwrap_or(true),
        rate_limit_max_requests: req.rate_limit_max_requests,
        rate_limit_window_seconds: req.rate_limit_window_seconds,
        methods: req.methods,
        timeout_ms: req.timeout_ms,
    };

    match state.service.create_route(cmd).await {
        Ok(route) => Ok(Json(serde_json::json!({
            "route": route.to_json(),
            "message": "Route created"
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn update_route(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(route_id): Path<String>,
    Json(req): Json<UpdateRouteDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can update routes
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let rid = Uuid::parse_str(&route_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid route ID", "code": "VALIDATION_ERROR"})),
        )
    })?;

    let cmd = UpdateRouteCommand {
        route_id: rid,
        path_prefix: req.path_prefix,
        target_service: req.target_service,
        target_url: req.target_url,
        auth_required: req.auth_required,
        rate_limit_max_requests: req.rate_limit_max_requests,
        rate_limit_window_seconds: req.rate_limit_window_seconds,
    };

    match state.service.update_route(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({"message": "Route updated"}))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn delete_route(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(route_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can delete routes
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let rid = Uuid::parse_str(&route_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid route ID", "code": "VALIDATION_ERROR"})),
        )
    })?;

    let cmd = DeleteRouteCommand { route_id: rid };

    match state.service.delete_route(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({"message": "Route deleted"}))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_route_health(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(route_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can view route health
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let rid = Uuid::parse_str(&route_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid route ID", "code": "VALIDATION_ERROR"})),
        )
    })?;

    let cmd = GetRouteHealthCommand { route_id: rid };

    match state.service.get_route_health(cmd).await {
        Ok(result) => match result.health {
            Some(h) => Ok(Json(h.to_json())),
            None => Ok(Json(serde_json::json!({"error": "Health data not available"}))),
        },
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_stats(
    State(state): State<AppState>,
    auth: AuthPrincipal,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can view stats
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    match state.service.get_stats().await {
        Ok(result) => Ok(Json(result.stats.to_json())),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

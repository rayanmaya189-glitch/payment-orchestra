use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::api::dto::*;
use crate::application::commands::*;
use crate::application::services::AiGatewayService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ai/guard", post(process_request))
        .route("/ai/requests/{request_id}", get(get_request))
        .route("/ai/requests", get(list_requests))
        .route("/ai/usage", get(get_usage_stats))
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
        PlatformError::RateLimited { retry_after_ms } => (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": e.to_string(),
                "code": "RATE_LIMITED",
                "retry_after_ms": retry_after_ms
            })),
        ),
        PlatformError::AuthorizationDenied(_) => (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": e.to_string(), "code": "AUTHORIZATION_DENIED"})),
        ),
        _ => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string(), "code": "INTERNAL_ERROR"})),
        ),
    }
}

async fn process_request(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<AiRequestDto>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    if req.prompt.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "prompt is required", "code": "VALIDATION_ERROR"})),
        ));
    }

    if req.prompt.len() > 10_000 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Prompt exceeds maximum length of 10,000 characters",
                "code": "VALIDATION_ERROR"
            })),
        ));
    }

    let cmd = AiGatewayRequest {
        principal_id: auth.principal_id,
        prompt: req.prompt,
        model: req.model,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
    };

    match state.service.process(cmd).await {
        Ok(r) => Ok(Json(r.response_summary())),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_request(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(request_id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let rid = Uuid::parse_str(&request_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid request ID", "code": "VALIDATION_ERROR"})),
        )
    })?;

    let cmd = GetRequestCommand {
        principal_id: auth.principal_id,
        request_id: rid,
    };

    match state.service.get_request(cmd).await {
        Ok(result) => match result.request {
            Some(r) => Ok(Json(r.response_summary())),
            None => Err((
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Request not found", "code": "NOT_FOUND"})),
            )),
        },
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn list_requests(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20)
        .min(100);

    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let cmd = ListRequestsCommand {
        principal_id: auth.principal_id,
        limit,
        offset,
    };

    match state.service.list_requests(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "data": result.requests.iter().map(|r| r.response_summary()).collect::<Vec<_>>(),
            "total": result.total
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_usage_stats(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = GetUsageStatsCommand {
        principal_id: auth.principal_id,
        start: params.get("start").cloned(),
        end: params.get("end").cloned(),
    };

    match state.service.get_usage_stats(cmd).await {
        Ok(result) => {
            let stats = &result.stats;
            Ok(Json(serde_json::json!({
                "total_requests": stats.total_requests,
                "blocked_requests": stats.blocked_requests,
                "block_rate": stats.block_rate(),
                "total_tokens": stats.total_tokens,
                "total_cost_usd": stats.total_cost_usd,
                "avg_latency_ms": stats.avg_latency_ms,
                "period_start": stats.period_start.to_rfc3339(),
                "period_end": stats.period_end.to_rfc3339()
            })))
        }
        Err(e) => Err(platform_error_to_response(e)),
    }
}

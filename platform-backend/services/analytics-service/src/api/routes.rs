use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::AnalyticsService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/analytics/payments", get(get_payment_analytics))
        .route("/analytics/operators/{operator_id}", get(get_operator_analytics))
        .route("/analytics/operators/{operator_id}/volume", get(get_volume_trend))
        .route("/analytics/operators/{operator_id}/declines", get(get_decline_breakdown))
        .route("/analytics/operators/{operator_id}/connectors", get(get_connector_performance))
        .route("/analytics/operators/{operator_id}/hourly", get(get_hourly_trends))
        .route("/analytics/operators/{operator_id}/events", get(get_authorization_events))
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

fn parse_operator_id(operator_id: &str) -> Result<Uuid, (axum::http::StatusCode, Json<serde_json::Value>)> {
    Uuid::parse_str(operator_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid operator ID", "code": "VALIDATION_ERROR"})),
        )
    })
}

async fn get_payment_analytics(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = GetPaymentAnalyticsCommand {
        operator_id: params
            .get("operator_id")
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or(auth.principal_id),
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_payment_analytics(cmd).await {
        Ok(result) => Ok(Json(result.analytics.to_json())),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_operator_analytics(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let cmd = GetOperatorAnalyticsCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_operator_analytics(cmd).await {
        Ok(result) => Ok(Json(result.analytics.to_summary_json())),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_volume_trend(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let cmd = GetVolumeTrendCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_volume_trend(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "buckets": result.buckets.iter().map(|b| serde_json::json!({
                "timestamp": b.timestamp.to_rfc3339(),
                "transaction_count": b.transaction_count,
                "volume": b.volume_minor_units,
                "success_count": b.success_count,
                "failure_count": b.failure_count,
                "avg_latency_ms": b.avg_latency_ms,
            })).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_decline_breakdown(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let cmd = GetDeclineBreakdownCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_decline_breakdown(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "declines": result.breakdown.iter().map(|d| serde_json::json!({
                "category": d.category.label(),
                "count": d.count,
                "percentage": d.percentage,
            })).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_connector_performance(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let cmd = GetConnectorPerformanceCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_connector_performance(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "connectors": result.connectors.iter().map(|c| serde_json::json!({
                "connector_id": c.connector_id,
                "total_requests": c.total_requests,
                "success_count": c.success_count,
                "failure_count": c.failure_count,
                "success_rate": c.success_rate,
                "avg_latency_ms": c.avg_latency_ms,
                "total_volume": c.total_volume,
            })).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_hourly_trends(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let cmd = GetVolumeTrendCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };

    match state.service.get_hourly_trends(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "hourly_trends": result.trends.iter().map(|t| serde_json::json!({
                "hour": t.hour.to_rfc3339(),
                "transaction_count": t.transaction_count,
                "volume": t.volume,
                "success_rate": t.success_rate,
            })).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

async fn get_authorization_events(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(operator_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = parse_operator_id(&operator_id)?;
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(100)
        .min(1000);

    let cmd = GetAuthorizationEventsCommand {
        operator_id: oid,
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
        limit,
    };

    match state.service.get_authorization_events(cmd).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "events": result.events.iter().map(|e| serde_json::json!({
                "event_id": e.event_id.to_string(),
                "payment_intent_id": e.payment_intent_id.to_string(),
                "status": e.status.label(),
                "amount": e.amount.amount_minor_units,
                "currency": e.amount.currency,
                "latency_ms": e.latency_ms,
                "occurred_at": e.occurred_at.to_rfc3339(),
            })).collect::<Vec<_>>()
        }))),
        Err(e) => Err(platform_error_to_response(e)),
    }
}

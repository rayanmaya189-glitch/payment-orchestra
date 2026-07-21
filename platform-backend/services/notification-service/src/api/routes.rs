use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::Deserialize;
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::NotificationService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/notifications", post(send_notification))
        .route("/notifications/{notification_id}", get(get_notification))
        .route("/notifications/{notification_id}/retry", post(retry_notification))
        .with_state(state)
}

#[derive(Deserialize)]
struct SendNotificationRequest { notification_type: String, recipient: String, subject: Option<String>, body: String }

async fn send_notification(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<SendNotificationRequest>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin, operator_admin, or api_client can send notifications
    if !matches!(auth.role.as_str(), "platform_admin" | "operator_admin" | "api_client") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    // Validate recipient is not empty
    if req.recipient.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Recipient is required", "code": "VALIDATION_ERROR"}))));
    }

    let cmd = SendNotificationCommand {
        operator_id: Uuid::now_v7(), // In production: derive from auth context
        notification_type: req.notification_type,
        recipient: req.recipient,
        subject: req.subject,
        body: req.body,
        template_id: None,
        template_data: None,
    };
    match state.service.send(cmd).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"notification_id": id.to_string()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_notification(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let nid = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid notification ID", "code": "INVALID_ID"})),
    ))?;

    // Only platform_admin and compliance_officer can view any notification
    // Other roles: would need operator-scoped check (not implemented yet)
    if !matches!(auth.role.as_str(), "platform_admin" | "compliance_officer") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    match state.service.get(nid).await {
        Ok(n) => Ok(Json(serde_json::json!({"notification_id": n.notification_id.to_string(), "status": n.status.as_str(), "type": n.notification_type.as_str()}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn retry_notification(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Only platform_admin can retry notifications
    if auth.role != "platform_admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only administrators can retry notifications", "code": "FORBIDDEN"}))));
    }

    let nid = Uuid::parse_str(&id).map_err(|_| (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Invalid notification ID", "code": "INVALID_ID"})),
    ))?;

    match state.service.retry(RetryNotificationCommand { notification_id: nid }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "retried"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

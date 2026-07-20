use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::Deserialize;
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::NotificationService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/notifications", post(send_notification))
        .route("/notifications/{notification_id}", get(get_notification))
        .route("/notifications/{notification_id}/retry", post(retry_notification))
        .with_state(state)
}

#[derive(Deserialize)]
struct SendNotificationRequest { notification_type: String, recipient: String, subject: Option<String>, body: String }

async fn send_notification(State(state): State<AppState>, Json(req): Json<SendNotificationRequest>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = SendNotificationCommand { operator_id: Uuid::nil(), notification_type: req.notification_type, recipient: req.recipient, subject: req.subject, body: req.body, template_id: None, template_data: None };
    match state.service.send(cmd).await {
        Ok(id) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"notification_id": id.to_string()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_notification(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let nid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get(nid).await {
        Ok(n) => Ok(Json(serde_json::json!({"notification_id": n.notification_id.to_string(), "status": n.status.as_str(), "type": n.notification_type.as_str()}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn retry_notification(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let nid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.retry(RetryNotificationCommand { notification_id: nid }).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "retried"}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

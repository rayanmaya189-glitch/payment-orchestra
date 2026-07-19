use axum::{extract::{Path, State}, http::StatusCode, Json, Router};
use uuid::Uuid;
use super::dto::*;
use super::AppState;
use crate::application::services::{NotificationService, NotificationResponse as ServiceResponse};

pub fn router(state: AppState) -> Router {
    Router::new().route("/notifications", axum::routing::post(send_notification)).route("/notifications/{id}", axum::routing::get(get_notification)).with_state(state)
}

async fn send_notification(State(state): State<AppState>, Json(req): Json<SendNotificationRequest>) -> Result<(StatusCode, Json<NotificationResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::SendNotificationCommand { operator_id: Uuid::nil(), notification_type: req.notification_type, recipient: req.recipient, subject: req.subject, body: req.body };
    match state.service.send_notification(cmd).await { Ok(r) => Ok((StatusCode::CREATED, Json(convert_response(r)))), Err(e) => Err(err(e)) }
}

async fn get_notification(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<NotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_notification(id).await { Ok(r) => Ok(Json(convert_response(r))), Err(e) => Err(err(e)) }
}

fn convert_response(r: ServiceResponse) -> NotificationResponse { NotificationResponse { notification_id: r.notification_id, status: r.status, notification_type: r.notification_type, recipient: r.recipient } }
fn err(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) { let (s,c,m) = match &e { platform_error::PlatformError::NotFound{resource,id} => (StatusCode::NOT_FOUND,"NOT_FOUND",format!("{resource} {id}")), _ => (StatusCode::INTERNAL_SERVER_ERROR,"INTERNAL_ERROR","Internal error".into()) }; (s, Json(ErrorResponse{error:m,code:c.to_string()})) }

use axum::{extract::{Path, State}, http::StatusCode, Json, Router};
use uuid::Uuid;
use super::dto::*;
use super::AppState;
use crate::application::services::{SubscriptionService, SubscriptionResponse as ServiceResponse};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/subscriptions", axum::routing::post(create_sub))
        .route("/subscriptions/{id}", axum::routing::get(get_sub))
        .route("/subscriptions/{id}/pause", axum::routing::post(pause_sub))
        .route("/subscriptions/{id}/resume", axum::routing::post(resume_sub))
        .route("/subscriptions/{id}/cancel", axum::routing::post(cancel_sub))
        .with_state(state)
}

async fn create_sub(State(state): State<AppState>, Json(req): Json<CreateSubscriptionRequest>) -> Result<(StatusCode, Json<SubscriptionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::CreateSubscriptionCommand {
        operator_id: Uuid::nil(),
        customer_id: req.customer_id,
        amount: req.amount,
        interval: req.interval,
    };
    match state.service.create_subscription(cmd).await {
        Ok(r) => Ok((StatusCode::CREATED, Json(convert_response(r)))),
        Err(e) => Err(err(e)),
    }
}

async fn get_sub(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<SubscriptionResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_subscription(id).await {
        Ok(r) => Ok(Json(convert_response(r))),
        Err(e) => Err(err(e)),
    }
}

async fn pause_sub(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.pause_subscription(id).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

async fn resume_sub(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.resume_subscription(id).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

async fn cancel_sub(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.cancel_subscription(id).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

fn convert_response(r: ServiceResponse) -> SubscriptionResponse {
    SubscriptionResponse {
        subscription_id: r.subscription_id,
        status: r.status,
        amount: r.amount,
        currency: r.currency,
        interval: r.interval,
        current_period_end: r.current_period_end,
    }
}

fn err(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (s, c, m) = match &e {
        platform_error::PlatformError::NotFound { resource, id } => (StatusCode::NOT_FOUND, "NOT_FOUND", format!("{resource} {id}")),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal error".into()),
    };
    (s, Json(ErrorResponse { error: m, code: c.to_string() }))
}

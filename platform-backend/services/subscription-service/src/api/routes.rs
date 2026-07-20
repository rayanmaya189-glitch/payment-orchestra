use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::SubscriptionService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/subscriptions", post(create_subscription).get(list_subscriptions))
        .route("/subscriptions/{subscription_id}", get(get_subscription))
        .route("/subscriptions/{subscription_id}/cancel", post(cancel_subscription))
        .with_state(state)
}

#[derive(Deserialize)]
struct CreateSubscriptionRequest {
    customer_id: String,
    amount_minor_units: i64,
    currency: String,
    interval: String,
    interval_count: Option<i32>,
    trial_period_days: Option<i32>,
    payment_method_token_id: Option<String>,
}

#[derive(Serialize)]
struct SubscriptionResponse {
    subscription_id: String,
    status: String,
    amount_minor_units: i64,
    currency: String,
    interval: String,
    current_period_end: String,
}

async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<(axum::http::StatusCode, Json<SubscriptionResponse>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = CreateSubscriptionCommand {
        operator_id: Uuid::nil(),
        customer_id: Uuid::parse_str(&req.customer_id).unwrap_or(Uuid::nil()),
        amount_minor_units: req.amount_minor_units,
        currency: req.currency,
        interval: req.interval,
        interval_count: req.interval_count,
        trial_period_days: req.trial_period_days,
        payment_method_token_id: req.payment_method_token_id,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.create_subscription(cmd).await {
        Ok(id) => {
            let sub = state.service.get_subscription(id).await.unwrap();
            Ok((
                axum::http::StatusCode::CREATED,
                Json(SubscriptionResponse {
                    subscription_id: sub.subscription_id.to_string(),
                    status: sub.status.as_str().to_string(),
                    amount_minor_units: sub.amount.amount_minor_units,
                    currency: sub.amount.currency.0.clone(),
                    interval: sub.interval.as_str().to_string(),
                    current_period_end: sub.current_period_end.to_rfc3339(),
                }),
            ))
        },
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn get_subscription(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SubscriptionResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get_subscription(sub_id).await {
        Ok(sub) => Ok(Json(SubscriptionResponse {
            subscription_id: sub.subscription_id.to_string(),
            status: sub.status.as_str().to_string(),
            amount_minor_units: sub.amount.amount_minor_units,
            currency: sub.amount.currency.0.clone(),
            interval: sub.interval.as_str().to_string(),
            current_period_end: sub.current_period_end.to_rfc3339(),
        })),
        Err(e) => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn list_subscriptions(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    Ok(Json(serde_json::json!({"data": [], "has_more": false})))
}

async fn cancel_subscription(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    let cmd = CancelSubscriptionCommand {
        subscription_id: sub_id,
        reason: "user_request".to_string(),
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };
    match state.service.cancel_subscription(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "canceled"}))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

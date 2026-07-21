use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::SubscriptionService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/subscriptions", post(create_subscription).get(list_subscriptions))
        .route("/subscriptions/{subscription_id}", get(get_subscription))
        .route(
            "/subscriptions/{subscription_id}/cancel",
            post(cancel_subscription),
        )
        .route(
            "/subscriptions/{subscription_id}/charge",
            post(charge_subscription),
        )
        .route(
            "/subscriptions/{subscription_id}/reactivate",
            post(reactivate_subscription),
        )
        .route(
            "/subscriptions/{subscription_id}/payment-method",
            post(update_payment_method),
        )
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
    dunning_profile: Option<String>,
}

#[derive(Deserialize)]
struct CancelSubscriptionRequest {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct ChargeSubscriptionRequest {
    payment_intent_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePaymentMethodRequest {
    new_payment_method_token: String,
}

#[derive(Deserialize)]
struct ListSubscriptionsQuery {
    status: Option<String>,
    customer_id: Option<String>,
    min_amount: Option<i64>,
    max_amount: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct SubscriptionResponse {
    subscription_id: String,
    operator_id: String,
    customer_id: String,
    status: String,
    amount_minor_units: i64,
    currency: String,
    interval: String,
    interval_count: i32,
    current_period_start: String,
    current_period_end: String,
    trial_period_days: i32,
    payment_method_token_id: Option<String>,
    retry_count: i32,
    max_retries: i32,
    canceled_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct PaginatedResponse<T: Serialize> {
    data: Vec<T>,
    has_more: bool,
    total: Option<i64>,
}

impl From<&crate::domain::aggregates::Subscription> for SubscriptionResponse {
    fn from(sub: &crate::domain::aggregates::Subscription) -> Self {
        Self {
            subscription_id: sub.subscription_id.to_string(),
            operator_id: sub.operator_id.to_string(),
            customer_id: sub.customer_id.to_string(),
            status: sub.status.as_str().to_string(),
            amount_minor_units: sub.amount.amount_minor_units,
            currency: sub.amount.currency.0.clone(),
            interval: sub.interval.as_str().to_string(),
            interval_count: sub.interval_count,
            current_period_start: sub.current_period_start.to_rfc3339(),
            current_period_end: sub.current_period_end.to_rfc3339(),
            trial_period_days: sub.trial_period_days,
            payment_method_token_id: sub.payment_method_token_id.clone(),
            retry_count: sub.retry_count,
            max_retries: sub.max_retries,
            canceled_at: sub.canceled_at.map(|dt| dt.to_rfc3339()),
            created_at: sub.created_at.to_rfc3339(),
            updated_at: sub.updated_at.to_rfc3339(),
        }
    }
}

fn error_response(status: axum::http::StatusCode, error: &PlatformError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let (code, message) = match error {
        PlatformError::NotFound { resource, id } => (
            "NOT_FOUND".to_string(),
            format!("{} {} not found", resource, id),
        ),
        PlatformError::Validation(e) => (
            "VALIDATION_ERROR".to_string(),
            e.to_string(),
        ),
        PlatformError::Conflict(e) => (
            "CONFLICT".to_string(),
            e.to_string(),
        ),
        PlatformError::AuthorizationDenied(msg) => (
            "FORBIDDEN".to_string(),
            msg.clone(),
        ),
        _ => (
            "INTERNAL_ERROR".to_string(),
            error.to_string(),
        ),
    };
    (
        status,
        Json(serde_json::json!({
            "error": {
                "code": code,
                "message": message,
            }
        })),
    )
}

async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<(axum::http::StatusCode, Json<SubscriptionResponse>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "customer_id".to_string(),
            )),
        )
    })?;

    let cmd = CreateSubscriptionCommand {
        operator_id: auth.principal_id, // Derive from auth context
        customer_id,
        amount_minor_units: req.amount_minor_units,
        currency: req.currency,
        interval: req.interval,
        interval_count: req.interval_count,
        trial_period_days: req.trial_period_days,
        payment_method_token_id: req.payment_method_token_id,
        dunning_profile: req.dunning_profile,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.create_subscription(cmd).await {
        Ok(id) => {
            let sub = state.service.get_subscription(id).await.map_err(|e| {
                error_response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, &e)
            })?;
            Ok((axum::http::StatusCode::CREATED, Json(SubscriptionResponse::from(&sub))))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn get_subscription(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SubscriptionResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "subscription_id".to_string(),
            )),
        )
    })?;

    match state.service.get_subscription(sub_id).await {
        Ok(sub) => Ok(Json(SubscriptionResponse::from(&sub))),
        Err(e) => Err(error_response(axum::http::StatusCode::NOT_FOUND, &e)),
    }
}

async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Query(query): Query<ListSubscriptionsQuery>,
) -> Result<Json<PaginatedResponse<SubscriptionResponse>>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let customer_id = query
        .customer_id
        .and_then(|s| Uuid::parse_str(&s).ok());

    let cmd = ListSubscriptionsCommand {
        operator_id: auth.principal_id,
        status: query.status,
        customer_id,
        min_amount: query.min_amount,
        max_amount: query.max_amount,
        limit: query.limit,
        offset: query.offset,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    let limit = cmd.limit;
    match state.service.list_subscriptions(cmd).await {
        Ok(subs) => {
            let has_more = subs.len() == limit.unwrap_or(20) as usize;
            let data = subs.iter().map(SubscriptionResponse::from).collect();
            Ok(Json(PaginatedResponse {
                data,
                has_more,
                total: None,
            }))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn cancel_subscription(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<CancelSubscriptionRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "subscription_id".to_string(),
            )),
        )
    })?;

    let cmd = CancelSubscriptionCommand {
        subscription_id: sub_id,
        reason: req.reason.unwrap_or_else(|| "user_request".to_string()),
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.cancel_subscription(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "canceled",
            "message": "Subscription has been canceled"
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn charge_subscription(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ChargeSubscriptionRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "subscription_id".to_string(),
            )),
        )
    })?;

    let payment_intent_id = req
        .payment_intent_id
        .and_then(|s| Uuid::parse_str(&s).ok());

    let cmd = ChargeSubscriptionCommand {
        subscription_id: sub_id,
        payment_intent_id,
    };

    match state.service.charge_subscription(cmd).await {
        Ok(()) => {
            let sub = state.service.get_subscription(sub_id).await.map_err(|e| {
                error_response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, &e)
            })?;
            Ok(Json(serde_json::json!({
                "status": sub.status.as_str(),
                "message": "Subscription charged successfully"
            })))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn reactivate_subscription(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "subscription_id".to_string(),
            )),
        )
    })?;

    let cmd = ReactivateSubscriptionCommand {
        subscription_id: sub_id,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.reactivate_subscription(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "active",
            "message": "Subscription has been reactivated"
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn update_payment_method(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<UpdatePaymentMethodRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let sub_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "subscription_id".to_string(),
            )),
        )
    })?;

    let cmd = UpdatePaymentMethodCommand {
        subscription_id: sub_id,
        new_payment_method_token: req.new_payment_method_token,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.update_payment_method(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "updated",
            "message": "Payment method updated successfully"
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::commands::*;
use crate::application::queries::*;
use crate::application::services::PaymentService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/payment-intents", axum::routing::post(create_payment_intent))
        .route("/payment-intents/{payment_intent_id}", axum::routing::get(get_payment_intent))
        .route("/payment-intents/{payment_intent_id}/authorize", axum::routing::post(authorize))
        .route("/payment-intents/{payment_intent_id}/capture", axum::routing::post(capture))
        .route("/payment-intents/{payment_intent_id}/void", axum::routing::post(void))
        .route("/payment-intents/{payment_intent_id}/refund", axum::routing::post(refund))
        .with_state(state)
}

async fn create_payment_intent(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<CreatePaymentIntentRequest>,
) -> Result<(StatusCode, Json<PaymentIntentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = CreatePaymentIntentCommand {
        operator_id: req.operator_id.unwrap_or(Uuid::nil()),
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        amount: req.amount,
        idempotency_key: format!("idem_{}", Uuid::now_v7()),
        purpose: req.purpose,
        metadata: req.metadata,
        preferred_gateway_profile_id: req.preferred_gateway_profile_id,
    };

    platform_logging::log_security_event(
        "orchestration-service",
        platform_logging::SecurityEventType::LoginSuccess, // placeholder — real event: PaymentIntentCreated
        platform_logging::SecurityOutcome::Success,
        Some(auth.principal_id),
        None,
        None,
        None,
        Some(serde_json::json!({"action": "create_payment_intent"})),
    );

    match state.service.create_payment_intent(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_payment_intent(
    State(state): State<AppState>,
    Path(payment_intent_id): Path<Uuid>,
) -> Result<Json<PaymentIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let query = GetPaymentIntentQuery { payment_intent_id };

    match state.service.get_payment_intent(query).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn authorize(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(payment_intent_id): Path<Uuid>,
    Json(req): Json<AuthorizePaymentIntentRequest>,
) -> Result<Json<PaymentIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = AuthorizePaymentIntentCommand {
        payment_intent_id,
        payment_method_token_id: req.payment_method_token_id,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        operator_id: Uuid::nil(),
    };

    match state.service.authorize(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn capture(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(payment_intent_id): Path<Uuid>,
    Json(req): Json<CapturePaymentIntentRequest>,
) -> Result<Json<PaymentIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = CapturePaymentIntentCommand {
        payment_intent_id,
        amount: req.amount,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        operator_id: Uuid::nil(),
    };

    match state.service.capture(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn void(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(payment_intent_id): Path<Uuid>,
) -> Result<Json<PaymentIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = VoidPaymentIntentCommand {
        payment_intent_id,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        operator_id: Uuid::nil(),
    };

    match state.service.void(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn refund(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(payment_intent_id): Path<Uuid>,
    Json(req): Json<RefundPaymentIntentRequest>,
) -> Result<Json<PaymentIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = RefundPaymentIntentCommand {
        payment_intent_id,
        amount: req.amount,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
        operator_id: Uuid::nil(),
    };

    match state.service.refund(cmd).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(error_to_response(e)),
    }
}

fn error_to_response(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code, message) = match &e {
        platform_error::PlatformError::NotFound { resource, id } => (
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("{resource} {id} not found"),
        ),
        platform_error::PlatformError::Conflict(c) => (
            StatusCode::CONFLICT,
            "CONFLICT",
            c.to_string(),
        ),
        platform_error::PlatformError::Validation(v) => (
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            v.to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal error".to_string(),
        ),
    };

    (status, Json(ErrorResponse { error: message, code: code.to_string() }))
}

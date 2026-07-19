use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::services::GatewayService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/gateway-profiles", axum::routing::post(create_profile))
        .route("/gateway-profiles/{profile_id}", axum::routing::get(get_profile))
        .route("/gateway-profiles/{profile_id}", axum::routing::put(update_profile))
        .route("/operators/{operator_id}/gateway-profiles", axum::routing::get(list_profiles))
        .route("/gateway-profiles/{profile_id}/validate", axum::routing::post(validate_transaction))
        .route("/operators/{operator_id}/select-gateway", axum::routing::post(select_gateway))
        .with_state(state)
}

async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateGatewayProfileRequest>,
) -> Result<(StatusCode, Json<GatewayProfileResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::CreateGatewayProfileCommand {
        operator_id: Uuid::nil(), // TODO: Get from auth context
        connector_id: req.connector_id,
        merchant_acquirer_link_id: req.merchant_acquirer_link_id,
        min_transaction_amount: req.min_transaction_amount,
        max_transaction_amount: req.max_transaction_amount,
        daily_volume_limit: req.daily_volume_limit,
        monthly_volume_limit: req.monthly_volume_limit,
        fixed_fee: req.fixed_fee,
        percentage_fee_bps: req.percentage_fee_bps,
        enabled_card_schemes: req.enabled_card_schemes,
        enabled_currencies: req.enabled_currencies,
        routing_priority: req.routing_priority,
    };

    match state.service.create_profile(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(GatewayProfileResponse {
            profile_id: response.profile_id,
            connector_id: response.connector_id,
            status: response.status,
            routing_priority: response.routing_priority,
            min_amount: response.min_amount,
            max_amount: response.max_amount,
            daily_volume_limit: response.daily_volume_limit,
            fixed_fee: response.fixed_fee,
            percentage_fee_bps: response.percentage_fee_bps,
        }))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<GatewayProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_profile(profile_id).await {
        Ok(response) => Ok(Json(GatewayProfileResponse {
            profile_id: response.profile_id,
            connector_id: response.connector_id,
            status: response.status,
            routing_priority: response.routing_priority,
            min_amount: response.min_amount,
            max_amount: response.max_amount,
            daily_volume_limit: response.daily_volume_limit,
            fixed_fee: response.fixed_fee,
            percentage_fee_bps: response.percentage_fee_bps,
        })),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn update_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<UpdateGatewayProfileRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::UpdateGatewayProfileCommand {
        profile_id,
        status: req.status,
        min_transaction_amount: req.min_transaction_amount,
        max_transaction_amount: req.max_transaction_amount,
        fixed_fee: req.fixed_fee,
        percentage_fee_bps: req.percentage_fee_bps,
        routing_priority: req.routing_priority,
    };

    match state.service.update_profile(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn list_profiles(
    State(state): State<AppState>,
    Path(operator_id): Path<Uuid>,
) -> Result<Json<Vec<GatewayProfileResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.list_profiles(operator_id).await {
        Ok(profiles) => Ok(Json(profiles.into_iter().map(|p| GatewayProfileResponse {
            profile_id: p.profile_id,
            connector_id: p.connector_id,
            status: p.status,
            routing_priority: p.routing_priority,
            min_amount: p.min_amount,
            max_amount: p.max_amount,
            daily_volume_limit: p.daily_volume_limit,
            fixed_fee: p.fixed_fee,
            percentage_fee_bps: p.percentage_fee_bps,
        }).collect())),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn validate_transaction(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<ValidateTransactionRequest>,
) -> Result<Json<ValidationResultResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::ValidateTransactionCommand {
        profile_id,
        amount: req.amount,
        card_scheme: req.card_scheme,
        currency: req.currency,
    };

    match state.service.validate_transaction(cmd).await {
        Ok(result) => Ok(Json(ValidationResultResponse {
            valid: result.valid,
            error: result.error,
            estimated_fee: result.estimated_fee,
        })),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn select_gateway(
    State(state): State<AppState>,
    Path(operator_id): Path<Uuid>,
    Json(req): Json<SelectGatewayRequest>,
) -> Result<Json<GatewaySelectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::SelectGatewayCommand {
        operator_id,
        amount: req.amount,
        card_scheme: req.card_scheme,
        currency: req.currency,
    };

    match state.service.select_gateway(cmd).await {
        Ok(selection) => Ok(Json(GatewaySelectionResponse {
            profile_id: selection.profile_id,
            connector_id: selection.connector_id,
            estimated_fee: selection.estimated_fee,
        })),
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
            "NO_ELIGIBLE_GATEWAY",
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

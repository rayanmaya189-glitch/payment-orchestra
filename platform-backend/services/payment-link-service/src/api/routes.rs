use axum::{extract::{Path, State}, http::StatusCode, Json, Router};
use uuid::Uuid;
use super::dto::*;
use super::AppState;
use crate::application::services::PaymentLinkService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/payment-links", axum::routing::post(create_link))
        .route("/payment-links/{link_id}", axum::routing::get(get_link))
        .route("/payment-links/{link_id}/deactivate", axum::routing::post(deactivate_link))
        .with_state(state)
}

async fn create_link(State(state): State<AppState>, _auth: AuthPrincipal, Json(req): Json<CreatePaymentLinkRequest>) -> Result<(StatusCode, Json<PaymentLinkResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::CreatePaymentLinkCommand { operator_id: Uuid::nil(), amount: req.amount, description: req.description, merchant_name: req.merchant_name, expires_at: None, max_uses: req.max_uses };
    match state.service.create_link(cmd).await {
        Ok(r) => Ok((StatusCode::CREATED, Json(PaymentLinkResponse { link_id: r.link_id, status: r.status, amount: r.amount, currency: r.currency, description: r.description, merchant_name: r.merchant_name, current_uses: r.current_uses, created_at: r.created_at }))),
        Err(e) => Err(err(e)),
    }
}

async fn get_link(State(state): State<AppState>, Path(link_id): Path<Uuid>) -> Result<Json<PaymentLinkResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_link(link_id).await {
        Ok(r) => Ok(Json(PaymentLinkResponse { link_id: r.link_id, status: r.status, amount: r.amount, currency: r.currency, description: r.description, merchant_name: r.merchant_name, current_uses: r.current_uses, created_at: r.created_at })),
        Err(e) => Err(err(e)),
    }
}

async fn deactivate_link(State(state): State<AppState>, Path(link_id): Path<Uuid>) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.deactivate_link(link_id).await { Ok(()) => Ok(StatusCode::OK), Err(e) => Err(err(e)) }
}

fn err(e: platform_error::PlatformError) -> (StatusCode, Json<ErrorResponse>) {
    let (s, c, m) = match &e { platform_error::PlatformError::NotFound{resource,id} => (StatusCode::NOT_FOUND,"NOT_FOUND",format!("{resource} {id}")), _ => (StatusCode::INTERNAL_SERVER_ERROR,"INTERNAL_ERROR","Internal error".into()) };
    (s, Json(ErrorResponse{error:m,code:c.to_string()}))
}

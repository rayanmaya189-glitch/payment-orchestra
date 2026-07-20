use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::PaymentLinkService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/payment-links", post(create_payment_link))
        .route("/payment-links/{link_id}", get(get_payment_link))
        .route("/payment-links/public/{token}", get(get_public_payment_link))
        .route("/payment-links/{token}/use", post(use_payment_link))
        .with_state(state)
}

async fn create_payment_link(State(state): State<AppState>, Json(req): Json<serde_json::Value>) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = CreatePaymentLinkCommand {
        operator_id: req["operator_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        description: req["description"].as_str().unwrap_or("").to_string(),
        merchant_name: req["merchant_name"].as_str().unwrap_or("").to_string(),
        amount_minor_units: req["amount_minor_units"].as_i64().unwrap_or(0),
        currency: req["currency"].as_str().unwrap_or("AED").to_string(),
        max_uses: req["max_uses"].as_i64().map(|v| v as i32),
        expires_in_hours: req["expires_in_hours"].as_i64(),
    };
    match state.service.create(cmd).await {
        Ok(link) => Ok((axum::http::StatusCode::CREATED, Json(serde_json::json!({"link_id": link.link_id.to_string(), "public_token": link.public_token, "status": link.status.as_str()})))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_payment_link(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let lid = Uuid::parse_str(&id).unwrap_or(Uuid::nil());
    match state.service.get_by_id(lid).await {
        Ok(l) => Ok(Json(serde_json::json!({"link_id": l.link_id.to_string(), "status": l.status.as_str(), "amount": l.amount.amount_minor_units, "uses": l.current_uses}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_public_payment_link(State(state): State<AppState>, Path(token): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match state.service.get_by_token(&token).await {
        Ok(l) => Ok(Json(serde_json::json!({"link_id": l.link_id.to_string(), "merchant_name": l.merchant_name, "amount": l.amount.amount_minor_units, "currency": l.amount.currency.0, "description": l.description, "valid": l.is_valid()}))),
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn use_payment_link(State(state): State<AppState>, Path(token): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match state.service.use_link(UsePaymentLinkCommand { public_token: token }).await {
        Ok(l) => Ok(Json(serde_json::json!({"link_id": l.link_id.to_string(), "status": l.status.as_str(), "current_uses": l.current_uses}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

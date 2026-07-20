use axum::{extract::{Path, State}, routing::get, Json, Router};
use uuid::Uuid;
use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::AnalyticsService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/analytics/payments", get(get_payment_analytics))
        .route("/analytics/operators/{operator_id}", get(get_operator_analytics))
        .with_state(state)
}

async fn get_payment_analytics(State(state): State<AppState>, axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let cmd = GetPaymentAnalyticsCommand {
        operator_id: params.get("operator_id").and_then(|s| Uuid::parse_str(s).ok()).unwrap_or(Uuid::nil()),
        start: params.get("start").cloned().unwrap_or_default(),
        end: params.get("end").cloned().unwrap_or_default(),
    };
    match state.service.get_payment_analytics(cmd).await {
        Ok(a) => Ok(Json(serde_json::json!({"total_volume": a.total_volume, "total_count": a.total_count, "success_rate": a.success_rate, "revenue": a.revenue}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

async fn get_operator_analytics(State(state): State<AppState>, Path(operator_id): Path<String>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let oid = Uuid::parse_str(&operator_id).unwrap_or(Uuid::nil());
    let cmd = GetPaymentAnalyticsCommand { operator_id: oid, start: "".to_string(), end: "".to_string() };
    match state.service.get_payment_analytics(cmd).await {
        Ok(a) => Ok(Json(serde_json::json!({"operator_id": operator_id, "total_volume": a.total_volume, "success_rate": a.success_rate}))),
        Err(e) => Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

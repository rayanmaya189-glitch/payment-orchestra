use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::services::ReconciliationService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/settlements/ingest", axum::routing::post(ingest_settlement))
        .route("/settlements/{batch_id}", axum::routing::get(get_batch))
        .route("/ledger/{transaction_id}/verify", axum::routing::get(verify_balance))
        .with_state(state)
}

async fn ingest_settlement(
    State(state): State<AppState>,
    Json(req): Json<IngestSettlementRequest>,
) -> Result<(StatusCode, Json<SettlementBatchResponse>), (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::IngestSettlementBatchCommand {
        operator_id: Uuid::nil(), // TODO: Get from auth context
        acquirer_link_id: req.acquirer_link_id,
        raw_file: Vec::new(), // TODO: Get from multipart
        file_format: req.file_format,
    };

    match state.service.ingest_settlement_batch(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(SettlementBatchResponse {
            settlement_batch_id: response.settlement_batch_id,
            status: response.status,
            total_records: response.total_records,
            matched_count: response.matched_count,
            unmatched_count: response.unmatched_count,
            total_amount: response.total_amount,
        }))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<SettlementBatchResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_batch(batch_id).await {
        Ok(response) => Ok(Json(SettlementBatchResponse {
            settlement_batch_id: response.settlement_batch_id,
            status: response.status,
            total_records: response.total_records,
            matched_count: response.matched_count,
            unmatched_count: response.unmatched_count,
            total_amount: response.total_amount,
        })),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn verify_balance(
    State(state): State<AppState>,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.verify_ledger_balance(transaction_id).await {
        Ok(balanced) => Ok(Json(serde_json::json!({
            "transaction_id": transaction_id,
            "balanced": balanced,
        }))),
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
            "DUPLICATE_BATCH",
            c.to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal error".to_string(),
        ),
    };

    (status, Json(ErrorResponse { error: message, code: code.to_string() }))
}

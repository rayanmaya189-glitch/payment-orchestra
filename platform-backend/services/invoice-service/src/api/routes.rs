use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use uuid::Uuid;

use super::dto::*;
use super::AppState;
use crate::application::services::InvoiceService;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/invoices", axum::routing::post(create_invoice))
        .route("/invoices/{invoice_id}", axum::routing::get(get_invoice))
        .route("/invoices/{invoice_id}/send", axum::routing::post(send_invoice))
        .route("/invoices/{invoice_id}/cancel", axum::routing::post(cancel_invoice))
        .route("/invoices/{invoice_id}/payments", axum::routing::post(record_payment))
        .with_state(state)
}

async fn create_invoice(
    State(state): State<AppState>,
    _auth: AuthPrincipal,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<InvoiceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let line_items = req.line_items.into_iter().map(|l| crate::domain::value_objects::InvoiceLineItem {
        description: l.description,
        amount_minor_units: l.amount_minor_units,
        quantity: l.quantity,
    }).collect();

    let due_date = chrono::DateTime::parse_from_rfc3339(&req.due_date)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now() + chrono::Duration::days(30));

    let cmd = crate::application::services::CreateInvoiceCommand {
        operator_id: Uuid::nil(), // TODO: Get from auth context
        order_reference: req.order_reference,
        line_items,
        due_date,
        recipient_email: req.recipient_email,
    };

    match state.service.create_invoice(cmd).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(InvoiceResponse {
            invoice_id: response.invoice_id,
            order_reference: response.order_reference,
            status: response.status,
            total_amount: response.total_amount,
            paid_amount: response.paid_amount,
            currency: response.currency,
            due_date: response.due_date,
            recipient_email: response.recipient_email,
        }))),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn get_invoice(
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.get_invoice(invoice_id).await {
        Ok(response) => Ok(Json(InvoiceResponse {
            invoice_id: response.invoice_id,
            order_reference: response.order_reference,
            status: response.status,
            total_amount: response.total_amount,
            paid_amount: response.paid_amount,
            currency: response.currency,
            due_date: response.due_date,
            recipient_email: response.recipient_email,
        })),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn send_invoice(
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::SendInvoiceCommand { invoice_id };

    match state.service.send_invoice(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn cancel_invoice(
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::CancelInvoiceCommand {
        invoice_id,
        reason: None,
    };

    match state.service.cancel_invoice(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err(error_to_response(e)),
    }
}

async fn record_payment(
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
    Json(req): Json<RecordPaymentRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let cmd = crate::application::services::RecordPaymentCommand {
        invoice_id,
        payment_intent_id: req.payment_intent_id,
        amount_minor_units: req.amount_minor_units,
    };

    match state.service.record_payment(cmd).await {
        Ok(()) => Ok(StatusCode::OK),
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
            "DUPLICATE_INVOICE",
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

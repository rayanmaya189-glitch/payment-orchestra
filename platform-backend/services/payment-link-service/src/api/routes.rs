use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::application::commands::*;
use crate::application::services::PaymentLinkService;
use platform_error::PlatformError;
use platform_middleware::AuthPrincipal;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/payment-links", post(create_payment_link).get(list_payment_links))
        .route("/payment-links/{link_id}", get(get_payment_link))
        .route(
            "/payment-links/{link_id}/deactivate",
            post(deactivate_payment_link),
        )
        .route(
            "/payment-links/{link_id}/metadata",
            post(update_payment_link_metadata),
        )
        .route("/payment-links/public/{token}", get(get_public_payment_link))
        .route("/payment-links/public/{token}/use", post(use_payment_link))
        .with_state(state)
}

#[derive(Deserialize)]
struct CreatePaymentLinkRequest {
    description: String,
    merchant_name: String,
    amount_minor_units: i64,
    currency: String,
    max_uses: Option<i32>,
    expires_in_hours: Option<i64>,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct UpdateMetadataRequest {
    metadata: serde_json::Value,
}

#[derive(Deserialize)]
struct ListPaymentLinksQuery {
    status: Option<String>,
    min_amount: Option<i64>,
    max_amount: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct PaymentLinkResponse {
    link_id: String,
    operator_id: String,
    status: String,
    description: String,
    merchant_name: String,
    amount_minor_units: i64,
    currency: String,
    max_uses: Option<i32>,
    current_uses: i32,
    remaining_uses: Option<i32>,
    expires_at: Option<String>,
    public_token: String,
    metadata: Option<serde_json::Value>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct PublicPaymentLinkResponse {
    link_id: String,
    merchant_name: String,
    amount_minor_units: i64,
    currency: String,
    description: String,
    valid: bool,
    remaining_uses: Option<i32>,
}

#[derive(Serialize)]
struct PaginatedResponse<T: Serialize> {
    data: Vec<T>,
    has_more: bool,
}

impl From<&crate::domain::aggregates::PaymentLink> for PaymentLinkResponse {
    fn from(link: &crate::domain::aggregates::PaymentLink) -> Self {
        Self {
            link_id: link.link_id.to_string(),
            operator_id: link.operator_id.to_string(),
            status: link.status.as_str().to_string(),
            description: link.description.clone(),
            merchant_name: link.merchant_name.clone(),
            amount_minor_units: link.amount.amount_minor_units,
            currency: link.amount.currency.0.clone(),
            max_uses: link.max_uses,
            current_uses: link.current_uses,
            remaining_uses: link.remaining_uses(),
            expires_at: link.expires_at.map(|dt| dt.to_rfc3339()),
            public_token: link.public_token.clone(),
            metadata: link.metadata.clone(),
            created_at: link.created_at.to_rfc3339(),
            updated_at: link.updated_at.to_rfc3339(),
        }
    }
}

fn error_response(
    status: axum::http::StatusCode,
    error: &PlatformError,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let (code, message) = match error {
        PlatformError::NotFound { resource, id } => (
            "NOT_FOUND".to_string(),
            format!("{} {} not found", resource, id),
        ),
        PlatformError::Validation(e) => ("VALIDATION_ERROR".to_string(), e.to_string()),
        PlatformError::Conflict(e) => ("CONFLICT".to_string(), e.to_string()),
        PlatformError::AuthorizationDenied(msg) => ("FORBIDDEN".to_string(), msg.clone()),
        _ => ("INTERNAL_ERROR".to_string(), "Internal error".to_string()),
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

async fn create_payment_link(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Json(req): Json<CreatePaymentLinkRequest>,
) -> Result<
    (axum::http::StatusCode, Json<PaymentLinkResponse>),
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    let cmd = CreatePaymentLinkCommand {
        operator_id: auth.principal_id,
        description: req.description,
        merchant_name: req.merchant_name,
        amount_minor_units: req.amount_minor_units,
        currency: req.currency,
        max_uses: req.max_uses,
        expires_in_hours: req.expires_in_hours,
        metadata: req.metadata,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.create(cmd).await {
        Ok(link) => Ok((
            axum::http::StatusCode::CREATED,
            Json(PaymentLinkResponse::from(&link)),
        )),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn get_payment_link(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<
    Json<PaymentLinkResponse>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    if !matches!(auth.role.as_str(), "platform_admin" | "operator_admin" | "compliance_officer") {
        return Err((axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Insufficient permissions", "code": "FORBIDDEN"}))));
    }

    let link_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "link_id".to_string(),
            )),
        )
    })?;

    match state.service.get_by_id(link_id).await {
        Ok(link) => Ok(Json(PaymentLinkResponse::from(&link))),
        Err(e) => Err(error_response(axum::http::StatusCode::NOT_FOUND, &e)),
    }
}

async fn get_public_payment_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<
    Json<PublicPaymentLinkResponse>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    match state.service.get_by_token(&token).await {
        Ok(link) => {
            let valid = link.is_valid();
            let remaining = link.remaining_uses();
            Ok(Json(PublicPaymentLinkResponse {
                link_id: link.link_id.to_string(),
                merchant_name: link.merchant_name,
                amount_minor_units: link.amount.amount_minor_units,
                currency: link.amount.currency.0,
                description: link.description,
                valid,
                remaining_uses: remaining,
            }))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::NOT_FOUND, &e)),
    }
}

async fn use_payment_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<
    Json<serde_json::Value>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    match state
        .service
        .use_link(UsePaymentLinkCommand {
            public_token: token,
        })
        .await
    {
        Ok(link) => Ok(Json(serde_json::json!({
            "link_id": link.link_id.to_string(),
            "status": link.status.as_str(),
            "current_uses": link.current_uses,
            "remaining_uses": link.remaining_uses(),
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn deactivate_payment_link(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
) -> Result<
    Json<serde_json::Value>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    let link_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "link_id".to_string(),
            )),
        )
    })?;

    let cmd = DeactivatePaymentLinkCommand {
        link_id,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.deactivate(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "deactivated",
            "message": "Payment link has been deactivated"
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn update_payment_link_metadata(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Path(id): Path<String>,
    Json(req): Json<UpdateMetadataRequest>,
) -> Result<
    Json<serde_json::Value>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    let link_id = Uuid::parse_str(&id).map_err(|_| {
        error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &PlatformError::Validation(platform_error::ValidationError::MissingField(
                "link_id".to_string(),
            )),
        )
    })?;

    let cmd = UpdatePaymentLinkMetadataCommand {
        link_id,
        metadata: req.metadata,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    match state.service.update_metadata(cmd).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "updated",
            "message": "Payment link metadata updated"
        }))),
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

async fn list_payment_links(
    State(state): State<AppState>,
    auth: AuthPrincipal,
    Query(query): Query<ListPaymentLinksQuery>,
) -> Result<
    Json<PaginatedResponse<PaymentLinkResponse>>,
    (axum::http::StatusCode, Json<serde_json::Value>),
> {
    let cmd = ListPaymentLinksCommand {
        operator_id: auth.principal_id,
        status: query.status,
        min_amount: query.min_amount,
        max_amount: query.max_amount,
        limit: query.limit,
        offset: query.offset,
        principal_id: auth.principal_id,
        role: auth.role.clone(),
    };

    let limit = cmd.limit;
    match state.service.list_links(cmd).await {
        Ok(links) => {
            let has_more = links.len() == limit.unwrap_or(20) as usize;
            let data = links.iter().map(PaymentLinkResponse::from).collect();
            Ok(Json(PaginatedResponse { data, has_more }))
        }
        Err(e) => Err(error_response(axum::http::StatusCode::BAD_REQUEST, &e)),
    }
}

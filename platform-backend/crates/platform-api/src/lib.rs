//! Shared OpenAPI/Swagger definitions for all services.
//!
//! Provides `ApiDoc` struct that aggregates all service APIs into a single
//! OpenAPI 3.0 specification, and a Swagger UI endpoint.

use utoipa::OpenApi;
use utoipa::ToSchema;

// Re-export utoipa for services to use
pub use utoipa;
pub use utoipa_axum::router::OpenApiRouter;

// ==================== Common Types ====================

/// Money value object (SRS Part 3 PRIN-004: integer minor units).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MoneySchema {
    /// Amount in minor units (e.g., cents)
    pub amount_minor_units: i64,
    /// ISO 4217 currency code (e.g., "AED")
    pub currency_code: String,
}

/// Standard error response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Paginated response wrapper.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

// ==================== Operator Service Schemas ====================

/// Operator registration request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterOperatorRequest {
    /// Legal company name
    pub legal_name: String,
    /// Trade license number
    pub trade_license_no: String,
    /// ISO 3166-1 alpha-2 country code
    pub country: String,
    /// Contact email
    pub email: String,
}

/// Operator response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct OperatorResponse {
    pub operator_id: String,
    pub legal_name: String,
    pub status: String,
    pub subdomain: String,
    pub country: String,
    pub created_at: String,
}

/// Operator status update request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateOperatorStatusRequest {
    /// New status: active_verified, suspended
    pub status: String,
    /// Reason for status change
    pub reason: Option<String>,
}

// ==================== IAM Service Schemas ====================

/// Login request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    /// User email
    pub email: String,
    /// Password (min 12 chars per SRS AUTH-003)
    pub password: String,
}

/// Login response with tokens.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Refresh token request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Create API key request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// API key response (secret shown once).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ApiKeyResponse {
    pub api_key_id: String,
    pub api_key_secret: String,
}

// ==================== Orchestration Service Schemas ====================

/// Create payment intent request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreatePaymentIntentRequest {
    pub amount: MoneySchema,
    pub idempotency_key: String,
    pub purpose: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub preferred_gateway_profile_id: Option<String>,
}

/// Payment intent response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PaymentIntentResponse {
    pub payment_intent_id: String,
    pub status: String,
    pub requested_amount: MoneySchema,
    pub authorized_amount: MoneySchema,
    pub captured_amount: MoneySchema,
    pub refunded_amount: MoneySchema,
    pub created_at: String,
}

/// Authorize payment intent request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct AuthorizePaymentIntentRequest {
    pub payment_method_token_id: String,
}

/// Capture payment intent request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CapturePaymentIntentRequest {
    pub amount: Option<MoneySchema>,
}

/// Refund payment intent request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct RefundPaymentIntentRequest {
    pub amount: MoneySchema,
}

// ==================== Connector Gateway Schemas ====================

/// Gateway profile response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct GatewayProfileResponse {
    pub profile_id: String,
    pub connector_id: String,
    pub status: String,
    pub base_url: String,
    pub routing_priority: i32,
}

/// Create gateway profile request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateGatewayProfileRequest {
    pub connector_id: String,
    pub merchant_acquirer_link_id: String,
    pub base_url: String,
    pub min_transaction_amount: Option<MoneySchema>,
    pub max_transaction_amount: Option<MoneySchema>,
    pub enabled_card_schemes: Vec<String>,
    pub enabled_currencies: Vec<String>,
    pub routing_priority: i32,
}

// ==================== Invoice Service Schemas ====================

/// Invoice line item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct InvoiceLineItemSchema {
    pub description: String,
    pub amount_minor_units: i64,
    pub quantity: Option<i32>,
}

/// Create invoice request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateInvoiceRequest {
    pub order_reference: String,
    pub line_items: Vec<InvoiceLineItemSchema>,
    pub due_date: String,
    pub recipient_email: Option<String>,
}

/// Invoice response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct InvoiceResponse {
    pub invoice_id: String,
    pub order_reference: String,
    pub status: String,
    pub total_amount: MoneySchema,
    pub paid_amount: MoneySchema,
    pub currency: String,
    pub due_date: String,
    pub recipient_email: Option<String>,
}

// ==================== KYB / Compliance Schemas ====================

/// Submit KYB evidence request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SubmitKybEvidenceRequest {
    pub document_ids: Vec<String>,
}

/// KYB case response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct KybCaseResponse {
    pub kyb_case_id: String,
    pub status: String,
    pub assigned_officer: Option<String>,
    pub risk_score: Option<f64>,
    pub created_at: String,
}

/// Decide KYB case request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct DecideKybCaseRequest {
    pub decision: String,
    pub reason: String,
}

// ==================== Saga / Reconciliation Schemas ====================

/// Saga instance response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SagaInstanceResponse {
    pub saga_id: String,
    pub saga_type: String,
    pub status: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub created_at: String,
}

/// Settlement batch response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SettlementBatchResponse {
    pub batch_id: String,
    pub status: String,
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount: MoneySchema,
}

// ==================== AI Service Schemas ====================

/// AI assistant query request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct AiQueryRequest {
    pub query: String,
    pub session_id: Option<String>,
}

/// AI assistant query response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AiQueryResponse {
    pub query_id: String,
    pub answer: String,
    pub citations: Vec<CitationSchema>,
    pub confidence: f64,
}

/// Citation source.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CitationSchema {
    pub source_type: String,
    pub source_id: String,
    pub text_snippet: String,
    pub relevance_score: f64,
}

// ==================== Document Service Schemas ====================

/// Document response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct DocumentResponse {
    pub document_id: String,
    pub document_type: String,
    pub filename: String,
    pub status: String,
    pub created_at: String,
}

// ==================== Notification Schemas ====================

/// Send notification request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SendNotificationRequest {
    pub notification_type: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
}

/// Notification response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct NotificationResponse {
    pub notification_id: String,
    pub status: String,
    pub notification_type: String,
    pub recipient: String,
}

// ==================== Risk Schemas ====================

/// Risk assessment response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RiskAssessmentResponse {
    pub assessment_id: String,
    pub score: f64,
    pub decision: String,
    pub factors: Vec<RiskFactorSchema>,
}

/// Risk factor.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RiskFactorSchema {
    pub factor: String,
    pub score: f64,
    pub weight: f64,
    pub description: String,
}

// ==================== Dispute Schemas ====================

/// Dispute response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct DisputeResponse {
    pub dispute_id: String,
    pub status: String,
    pub reason: String,
    pub amount: MoneySchema,
    pub currency: String,
}

// ==================== Subscription Schemas ====================

/// Subscription response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SubscriptionResponse {
    pub subscription_id: String,
    pub status: String,
    pub amount: MoneySchema,
    pub interval: String,
    pub current_period_end: String,
}

/// Create subscription request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateSubscriptionRequest {
    pub customer_id: String,
    pub amount: MoneySchema,
    pub interval: String,
}

// ==================== Payment Link Schemas ====================

/// Payment link response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PaymentLinkResponse {
    pub link_id: String,
    pub status: String,
    pub amount: MoneySchema,
    pub description: String,
    pub merchant_name: String,
    pub created_at: String,
}

/// Create payment link request.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct CreatePaymentLinkRequest {
    pub amount: MoneySchema,
    pub description: String,
    pub merchant_name: String,
    pub max_uses: Option<i32>,
}

// ==================== Health Check Schemas ====================

/// Health check response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct HealthCheckResponse {
    pub status: String,
    pub checks: Option<serde_json::Value>,
}

// ==================== OpenAPI Document ====================

/// Master OpenAPI document aggregating all service APIs.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Payment Orchestration Platform API",
        version = "1.0.0",
        description = "AI-Native Payment Orchestration Platform — Single-tenant, event-sourced, CQRS",
        contact(name = "Platform Team"),
        license(name = "Proprietary")
    ),
    paths(
        // Operator Service
        crate::operator_register,
        crate::operator_list,
        crate::operator_get,
        crate::operator_verify_email,
        crate::operator_update_status,
        // IAM Service
        crate::iam_login,
        crate::iam_refresh,
        crate::iam_create_api_key,
        crate::iam_revoke_api_key,
        // Orchestration Service
        crate::orchestration_create_intent,
        crate::orchestration_get_intent,
        crate::orchestration_authorize,
        crate::orchestration_capture,
        crate::orchestration_void,
        crate::orchestration_refund,
        // Connector Gateway
        crate::connector_create_profile,
        crate::connector_list_profiles,
        crate::connector_get_profile,
        // Invoice Service
        crate::invoice_create,
        crate::invoice_get,
        crate::invoice_send,
        crate::invoice_cancel,
        // Compliance Service
        crate::compliance_submit_kyb,
        crate::compliance_review_kyb,
        crate::compliance_get_case,
        // Health
        crate::health_check,
    ),
    components(schemas(
        MoneySchema, ErrorResponse, HealthCheckResponse,
        RegisterOperatorRequest, OperatorResponse, UpdateOperatorStatusRequest,
        LoginRequest, LoginResponse, RefreshTokenRequest,
        CreateApiKeyRequest, ApiKeyResponse,
        CreatePaymentIntentRequest, PaymentIntentResponse,
        AuthorizePaymentIntentRequest, CapturePaymentIntentRequest, RefundPaymentIntentRequest,
        GatewayProfileResponse, CreateGatewayProfileRequest,
        InvoiceLineItemSchema, CreateInvoiceRequest, InvoiceResponse,
        SubmitKybEvidenceRequest, KybCaseResponse, DecideKybCaseRequest,
        SagaInstanceResponse, SettlementBatchResponse,
        AiQueryRequest, AiQueryResponse, CitationSchema,
        DocumentResponse,
        SendNotificationRequest, NotificationResponse,
        RiskAssessmentResponse, RiskFactorSchema,
        DisputeResponse,
        SubscriptionResponse, CreateSubscriptionRequest,
        PaymentLinkResponse, CreatePaymentLinkRequest,
    )),
    tags(
        (name = "Operators", description = "Operator management"),
        (name = "Authentication", description = "IAM and authentication"),
        (name = "Payments", description = "Payment orchestration"),
        (name = "Connectors", description = "Gateway connector management"),
        (name = "Invoices", description = "Invoice management"),
        (name = "Compliance", description = "KYB and compliance"),
        (name = "Health", description = "Service health checks"),
    )
)]
pub struct ApiDoc;

// ==================== Route Handlers ====================

use axum::{extract::Path, Json};

// --- Operator Service ---

#[utoipa::path(
    post,
    path = "/v1/operators",
    tag = "Operators",
    request_body = RegisterOperatorRequest,
    responses(
        (status = 201, description = "Operator created", body = OperatorResponse),
        (status = 409, description = "Duplicate trade license", body = ErrorResponse)
    )
)]
pub async fn operator_register(
    Json(_req): Json<RegisterOperatorRequest>,
) -> Json<OperatorResponse> {
    Json(OperatorResponse {
        operator_id: "todo".to_string(),
        legal_name: "todo".to_string(),
        status: "pending".to_string(),
        subdomain: "todo".to_string(),
        country: "AE".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/v1/operators",
    tag = "Operators",
    responses(
        (status = 200, description = "List of operators")
    )
)]
pub async fn operator_list() -> Json<serde_json::Value> {
    Json(serde_json::json!({"data": [], "has_more": false}))
}

#[utoipa::path(
    get,
    path = "/v1/operators/{operator_id}",
    tag = "Operators",
    params(
        ("operator_id" = String, Path, description = "Operator ID")
    ),
    responses(
        (status = 200, description = "Operator found", body = OperatorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
pub async fn operator_get(
    Path(_id): Path<String>,
) -> Json<OperatorResponse> {
    Json(OperatorResponse {
        operator_id: "todo".to_string(),
        legal_name: "todo".to_string(),
        status: "active_verified".to_string(),
        subdomain: "todo".to_string(),
        country: "AE".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/operators/{operator_id}/verify-email",
    tag = "Operators",
    params(
        ("operator_id" = String, Path, description = "Operator ID")
    ),
    responses(
        (status = 200, description = "Email verified"),
        (status = 403, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn operator_verify_email(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "active_unverified"}))
}

#[utoipa::path(
    put,
    path = "/v1/operators/{operator_id}/status",
    tag = "Operators",
    params(
        ("operator_id" = String, Path, description = "Operator ID")
    ),
    request_body = UpdateOperatorStatusRequest,
    responses(
        (status = 200, description = "Status updated"),
        (status = 403, description = "Invalid transition", body = ErrorResponse)
    )
)]
pub async fn operator_update_status(
    Path(_id): Path<String>,
    Json(_req): Json<UpdateOperatorStatusRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "updated"}))
}

// --- IAM Service ---

#[utoipa::path(
    post,
    path = "/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse)
    )
)]
pub async fn iam_login(
    Json(_req): Json<LoginRequest>,
) -> Json<LoginResponse> {
    Json(LoginResponse {
        access_token: "todo".to_string(),
        refresh_token: "todo".to_string(),
        expires_in: 900,
    })
}

#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = LoginResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse)
    )
)]
pub async fn iam_refresh(
    Json(_req): Json<RefreshTokenRequest>,
) -> Json<LoginResponse> {
    Json(LoginResponse {
        access_token: "todo".to_string(),
        refresh_token: "todo".to_string(),
        expires_in: 900,
    })
}

#[utoipa::path(
    post,
    path = "/v1/principals/{principal_id}/api-keys",
    tag = "Authentication",
    params(
        ("principal_id" = String, Path, description = "Principal ID")
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created", body = ApiKeyResponse),
        (status = 403, description = "Maker/Checker required", body = ErrorResponse)
    )
)]
pub async fn iam_create_api_key(
    Path(_id): Path<String>,
    Json(_req): Json<CreateApiKeyRequest>,
) -> Json<ApiKeyResponse> {
    Json(ApiKeyResponse {
        api_key_id: "todo".to_string(),
        api_key_secret: "todo".to_string(),
    })
}

#[utoipa::path(
    delete,
    path = "/v1/api-keys/{api_key_id}",
    tag = "Authentication",
    params(
        ("api_key_id" = String, Path, description = "API Key ID")
    ),
    responses(
        (status = 200, description = "API key revoked")
    )
)]
pub async fn iam_revoke_api_key(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "revoked"}))
}

// --- Orchestration Service ---

#[utoipa::path(
    post,
    path = "/v1/payment-intents",
    tag = "Payments",
    request_body = CreatePaymentIntentRequest,
    responses(
        (status = 201, description = "Payment intent created", body = PaymentIntentResponse),
        (status = 409, description = "Idempotency conflict", body = ErrorResponse)
    )
)]
pub async fn orchestration_create_intent(
    Json(_req): Json<CreatePaymentIntentRequest>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "created".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/v1/payment-intents/{payment_intent_id}",
    tag = "Payments",
    params(
        ("payment_intent_id" = String, Path, description = "Payment Intent ID")
    ),
    responses(
        (status = 200, description = "Payment intent", body = PaymentIntentResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
pub async fn orchestration_get_intent(
    Path(_id): Path<String>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "created".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/payment-intents/{payment_intent_id}/authorize",
    tag = "Payments",
    params(
        ("payment_intent_id" = String, Path, description = "Payment Intent ID")
    ),
    request_body = AuthorizePaymentIntentRequest,
    responses(
        (status = 200, description = "Authorized", body = PaymentIntentResponse),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn orchestration_authorize(
    Path(_id): Path<String>,
    Json(_req): Json<AuthorizePaymentIntentRequest>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "authorized".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/payment-intents/{payment_intent_id}/capture",
    tag = "Payments",
    params(
        ("payment_intent_id" = String, Path, description = "Payment Intent ID")
    ),
    request_body = CapturePaymentIntentRequest,
    responses(
        (status = 200, description = "Captured", body = PaymentIntentResponse),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn orchestration_capture(
    Path(_id): Path<String>,
    Json(_req): Json<CapturePaymentIntentRequest>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "captured".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/payment-intents/{payment_intent_id}/void",
    tag = "Payments",
    params(
        ("payment_intent_id" = String, Path, description = "Payment Intent ID")
    ),
    responses(
        (status = 200, description = "Voided", body = PaymentIntentResponse),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn orchestration_void(
    Path(_id): Path<String>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "voided".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/payment-intents/{payment_intent_id}/refund",
    tag = "Payments",
    params(
        ("payment_intent_id" = String, Path, description = "Payment Intent ID")
    ),
    request_body = RefundPaymentIntentRequest,
    responses(
        (status = 200, description = "Refunded", body = PaymentIntentResponse),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn orchestration_refund(
    Path(_id): Path<String>,
    Json(_req): Json<RefundPaymentIntentRequest>,
) -> Json<PaymentIntentResponse> {
    Json(PaymentIntentResponse {
        payment_intent_id: "todo".to_string(),
        status: "refunded".to_string(),
        requested_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        authorized_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        captured_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        refunded_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

// --- Connector Gateway ---

#[utoipa::path(
    post,
    path = "/v1/gateway-profiles",
    tag = "Connectors",
    request_body = CreateGatewayProfileRequest,
    responses(
        (status = 201, description = "Profile created", body = GatewayProfileResponse)
    )
)]
pub async fn connector_create_profile(
    Json(_req): Json<CreateGatewayProfileRequest>,
) -> Json<GatewayProfileResponse> {
    Json(GatewayProfileResponse {
        profile_id: "todo".to_string(),
        connector_id: "todo".to_string(),
        status: "active".to_string(),
        base_url: "todo".to_string(),
        routing_priority: 1,
    })
}

#[utoipa::path(
    get,
    path = "/v1/operators/{operator_id}/gateway-profiles",
    tag = "Connectors",
    params(
        ("operator_id" = String, Path, description = "Operator ID")
    ),
    responses(
        (status = 200, description = "List of gateway profiles")
    )
)]
pub async fn connector_list_profiles(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"data": [], "has_more": false}))
}

#[utoipa::path(
    get,
    path = "/v1/gateway-profiles/{profile_id}",
    tag = "Connectors",
    params(
        ("profile_id" = String, Path, description = "Gateway Profile ID")
    ),
    responses(
        (status = 200, description = "Gateway profile", body = GatewayProfileResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
pub async fn connector_get_profile(
    Path(_id): Path<String>,
) -> Json<GatewayProfileResponse> {
    Json(GatewayProfileResponse {
        profile_id: "todo".to_string(),
        connector_id: "todo".to_string(),
        status: "active".to_string(),
        base_url: "todo".to_string(),
        routing_priority: 1,
    })
}

// --- Invoice Service ---

#[utoipa::path(
    post,
    path = "/v1/invoices",
    tag = "Invoices",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 201, description = "Invoice created", body = InvoiceResponse),
        (status = 409, description = "Duplicate order reference", body = ErrorResponse)
    )
)]
pub async fn invoice_create(
    Json(_req): Json<CreateInvoiceRequest>,
) -> Json<InvoiceResponse> {
    Json(InvoiceResponse {
        invoice_id: "todo".to_string(),
        order_reference: "todo".to_string(),
        status: "draft".to_string(),
        total_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        paid_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        currency: "AED".into(),
        due_date: "2026-02-01".to_string(),
        recipient_email: None,
    })
}

#[utoipa::path(
    get,
    path = "/v1/invoices/{invoice_id}",
    tag = "Invoices",
    params(
        ("invoice_id" = String, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice", body = InvoiceResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
pub async fn invoice_get(
    Path(_id): Path<String>,
) -> Json<InvoiceResponse> {
    Json(InvoiceResponse {
        invoice_id: "todo".to_string(),
        order_reference: "todo".to_string(),
        status: "draft".to_string(),
        total_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        paid_amount: MoneySchema { amount_minor_units: 0, currency_code: "AED".into() },
        currency: "AED".into(),
        due_date: "2026-02-01".to_string(),
        recipient_email: None,
    })
}

#[utoipa::path(
    post,
    path = "/v1/invoices/{invoice_id}/send",
    tag = "Invoices",
    params(
        ("invoice_id" = String, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice sent"),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn invoice_send(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "sent"}))
}

#[utoipa::path(
    post,
    path = "/v1/invoices/{invoice_id}/cancel",
    tag = "Invoices",
    params(
        ("invoice_id" = String, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice cancelled"),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn invoice_cancel(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "cancelled"}))
}

// --- Compliance Service ---

#[utoipa::path(
    post,
    path = "/v1/kyb-cases",
    tag = "Compliance",
    request_body = SubmitKybEvidenceRequest,
    responses(
        (status = 201, description = "KYB case created", body = KybCaseResponse)
    )
)]
pub async fn compliance_submit_kyb(
    Json(_req): Json<SubmitKybEvidenceRequest>,
) -> Json<KybCaseResponse> {
    Json(KybCaseResponse {
        kyb_case_id: "todo".to_string(),
        status: "submitted".to_string(),
        assigned_officer: None,
        risk_score: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/kyb-cases/{kyb_case_id}/decide",
    tag = "Compliance",
    params(
        ("kyb_case_id" = String, Path, description = "KYB Case ID")
    ),
    request_body = DecideKybCaseRequest,
    responses(
        (status = 200, description = "Case decided"),
        (status = 409, description = "Invalid state", body = ErrorResponse)
    )
)]
pub async fn compliance_review_kyb(
    Path(_id): Path<String>,
    Json(_req): Json<DecideKybCaseRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "approved"}))
}

#[utoipa::path(
    get,
    path = "/v1/kyb-cases/{kyb_case_id}",
    tag = "Compliance",
    params(
        ("kyb_case_id" = String, Path, description = "KYB Case ID")
    ),
    responses(
        (status = 200, description = "KYB case", body = KybCaseResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
pub async fn compliance_get_case(
    Path(_id): Path<String>,
) -> Json<KybCaseResponse> {
    Json(KybCaseResponse {
        kyb_case_id: "todo".to_string(),
        status: "under_review".to_string(),
        assigned_officer: None,
        risk_score: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
    })
}

// --- Health ---

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthCheckResponse)
    )
)]
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "ok".to_string(),
        checks: None,
    })
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_doc_generation() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "Payment Orchestration Platform API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[test]
    fn test_openapi_json_output() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string_pretty(&spec).unwrap();
        assert!(json.contains("Payment Orchestration Platform API"));
        assert!(json.contains("/v1/operators"));
        assert!(json.contains("/v1/payment-intents"));
        assert!(json.contains("/v1/auth/login"));
    }

    #[test]
    fn test_schemas_defined() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_value(&spec).unwrap();
        // Check that key schemas exist
        assert!(json["components"]["schemas"]["MoneySchema"].is_object());
        assert!(json["components"]["schemas"]["LoginRequest"].is_object());
        assert!(json["components"]["schemas"]["PaymentIntentResponse"].is_object());
    }
}

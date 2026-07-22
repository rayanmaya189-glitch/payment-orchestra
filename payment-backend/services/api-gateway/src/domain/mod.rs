//! API Gateway domain model — BC-17
//!
//! Single ingress for all external REST+protobuf traffic.
//! TLS termination, auth, rate limiting, CORS, request routing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Route — maps REST path + method to internal gRPC service
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    pub route_id: Uuid,
    pub http_method: String,
    pub url_pattern: String,
    pub grpc_service: String,
    pub grpc_method: String,
    pub path_params: Vec<String>,
    pub rate_limit_config: Option<RateLimitConfig>,
    pub auth_required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u32,
    pub per: RateLimitScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateLimitScope {
    Ip,
    ApiKey,
    Operator,
}

// ---------------------------------------------------------------------------
// ApiRequest — an inbound request to the gateway
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub request_id: Uuid,
    pub http_method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub source_ip: String,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedRequest {
    pub request_id: Uuid,
    pub route: Option<RouteDefinition>,
    pub authenticated: bool,
    pub actor_id: Option<Uuid>,
    pub actor_type: Option<String>,
    pub rate_limited: bool,
    pub allowed: bool,
    pub http_status: u16,
    pub response_body: Vec<u8>,
    pub processed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Authentication
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub authenticated: bool,
    pub actor_id: Option<Uuid>,
    pub actor_type: Option<ActorType>,
    pub scopes: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    User,
    ApiKey,
    System,
}

// ---------------------------------------------------------------------------
// Default routes — maps REST endpoints to internal gRPC methods
// ---------------------------------------------------------------------------

pub fn default_routes() -> Vec<RouteDefinition> {
    vec![
        // Payment Intents
        route("POST", "/v1/payment-intents", "orchestration.v1.OrchestrationService", "CreatePaymentIntent", vec![], Some(1000), true),
        route("POST", "/v1/payment-intents/*/authorize", "orchestration.v1.OrchestrationService", "AuthorizePaymentIntent", vec!["id".into()], Some(1000), true),
        route("POST", "/v1/payment-intents/*/capture", "orchestration.v1.OrchestrationService", "CapturePaymentIntent", vec!["id".into()], Some(500), true),
        route("POST", "/v1/payment-intents/*/refund", "orchestration.v1.OrchestrationService", "RefundPaymentIntent", vec!["id".into()], Some(200), true),
        route("POST", "/v1/payment-intents/*/void", "orchestration.v1.OrchestrationService", "VoidPaymentIntent", vec!["id".into()], Some(500), true),
        route("POST", "/v1/payment-intents/search", "orchestration.v1.OrchestrationService", "SearchPaymentIntents", vec![], Some(500), true),
        route("PATCH", "/v1/payment-intents/*", "orchestration.v1.OrchestrationService", "UpdatePaymentIntent", vec!["id".into()], Some(500), true),
        route("DELETE", "/v1/payment-intents/*", "orchestration.v1.OrchestrationService", "VoidPaymentIntent", vec!["id".into()], Some(500), true),

        // Invoices
        route("POST", "/v1/invoices", "invoice.v1.InvoiceService", "CreateInvoice", vec![], Some(50), true),
        route("POST", "/v1/invoices/search", "invoice.v1.InvoiceService", "SearchInvoices", vec![], Some(100), true),

        // Subscriptions
        route("POST", "/v1/subscriptions", "subscription.v1.SubscriptionService", "CreateSubscription", vec![], Some(50), true),
        route("POST", "/v1/subscriptions/search", "subscription.v1.SubscriptionService", "SearchSubscriptions", vec![], Some(100), true),

        // Connectors
        route("POST", "/v1/connectors", "connector.v1.ConnectorService", "ListConnectors", vec![], Some(100), true),
        route("POST", "/v1/merchant-links", "merchant.v1.MerchantService", "CreateMerchantLink", vec![], Some(20), true),
        route("POST", "/v1/merchant-links/*/test", "merchant.v1.MerchantService", "TestConnection", vec!["id".into()], Some(20), true),

        // Analytics
        route("POST", "/v1/analytics/transactions/search", "analytics.v1.AnalyticsService", "SearchTransactions", vec![], Some(100), true),
        route("POST", "/v1/analytics/fees/search", "analytics.v1.AnalyticsService", "SearchFees", vec![], Some(100), true),

        // Authentication
        route("POST", "/v1/authenticate", "iam.v1.IamService", "Authenticate", vec![], Some(10), false),

        // Webhooks
        route("POST", "/v1/webhook-endpoints", "webhook.v1.WebhookService", "CreateWebhookEndpoint", vec![], Some(10), true),
        route("POST", "/v1/webhook-endpoints/search", "webhook.v1.WebhookService", "ListWebhookEndpoints", vec![], Some(50), true),

        // AI Assistant
        route("POST", "/v1/ai/ask", "ai_assistant.v1.AiAssistantService", "AskQuestion", vec![], Some(30), true),
        route("POST", "/v1/ai/sessions", "ai_assistant.v1.AiAssistantService", "StartConversation", vec![], Some(30), true),
    ]
}

fn route(
    method: &str, pattern: &str, service: &str, grpc_method: &str,
    params: Vec<String>, rate_limit: Option<u32>, auth: bool,
) -> RouteDefinition {
    RouteDefinition {
        route_id: Uuid::now_v7(),
        http_method: method.into(),
        url_pattern: pattern.into(),
        grpc_service: service.into(),
        grpc_method: grpc_method.into(),
        path_params: params,
        rate_limit_config: rate_limit.map(|max| RateLimitConfig {
            max_requests: max,
            window_seconds: 60,
            per: RateLimitScope::ApiKey,
        }),
        auth_required: auth,
        description: format!("{} {}", method, pattern),
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum GatewayError {
    #[error("Route not found: {0} {1}")]
    RouteNotFound(String, String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),
    #[error("Backend service error: {0}")]
    BackendError(String),
    #[error("Request body too large: {size} bytes (max: {max})")]
    BodyTooLarge { size: usize, max: usize },
    #[error("CORS origin not allowed: {0}")]
    CorsOriginNotAllowed(String),
    #[error("Internal gateway error: {0}")]
    InternalError(String),
}

// ---------------------------------------------------------------------------
// Security headers
// ---------------------------------------------------------------------------

pub fn security_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Strict-Transport-Security", "max-age=31536000; includeSubDomains; preload"),
        ("X-Content-Type-Options", "nosniff"),
        ("X-Frame-Options", "DENY"),
        ("Referrer-Policy", "strict-origin-when-cross-origin"),
        ("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=()"),
        ("X-XSS-Protection", "0"),
    ]
}

// ---------------------------------------------------------------------------
// CORS
// ---------------------------------------------------------------------------

pub const ALLOWED_ORIGINS: &[&str] = &[
    "https://dashboard.paymentorchestra.com",
    "https://api.paymentorchestra.com",
];

pub const ALLOWED_METHODS: &[&str] = &["POST", "PATCH", "DELETE", "OPTIONS"];

pub const ALLOWED_HEADERS: &[&str] = &[
    "Content-Type", "Authorization", "X-API-Key", "X-Idempotency-Key",
    "X-Request-ID", "X-CSRF-Token",
];

pub const MAX_BODY_SIZE: usize = 1_048_576; // 1MB

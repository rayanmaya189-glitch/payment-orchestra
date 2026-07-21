#![allow(dead_code, unused_imports)]
//! Shared Axum middleware for all services.
//!
//! Provides: auth extraction, CORS, security headers, request ID, rate limiting, SSRF protection, graceful shutdown, API versioning.

pub mod abac;
pub mod api_version;
pub mod auth;
pub mod cors;
pub mod headers;
pub mod idempotency;
pub mod metrics;
pub mod openapi;
pub mod request_id;
pub mod rate_limit;
pub mod ssrf;
pub mod shutdown;

pub use abac::{evaluate_policy, check_maker_checker, requires_dual_control, AbacContext};
pub use api_version::ApiVersionLayer;
pub use auth::{AuthPrincipal, AuthMethod, JwtAuthLayer, ApiKeyAuthLayer, client_fingerprint};
pub use cors::cors_layer;
pub use headers::SecurityHeadersLayer;
pub use idempotency::{check_idempotency, store_idempotency, claim_idempotency};
pub use metrics::{MetricsLayer, MetricsState, metrics_handler};
pub use request_id::RequestIdLayer;
pub use rate_limit::{RateLimitLayer, RateLimitLayerConfig};
pub use ssrf::{validate_url, SsrfCheckResult};
pub use shutdown::{shutdown_signal, ShutdownConfig};

//! Shared Axum middleware for all services.
//!
//! Provides: auth extraction, CORS, security headers, request ID, rate limiting, SSRF protection, graceful shutdown, API versioning.

pub mod api_version;
pub mod auth;
pub mod cors;
pub mod headers;
pub mod metrics;
pub mod openapi;
pub mod request_id;
pub mod rate_limit;
pub mod ssrf;
pub mod shutdown;

pub use api_version::ApiVersionLayer;
pub use auth::{AuthPrincipal, AuthMethod, JwtAuthLayer, ApiKeyAuthLayer, client_fingerprint};
pub use cors::cors_layer;
pub use headers::SecurityHeadersLayer;
pub use metrics::{MetricsLayer, MetricsState, metrics_handler};
pub use request_id::RequestIdLayer;
pub use rate_limit::{RateLimitLayer, RateLimitLayerConfig};
pub use ssrf::{validate_url, SsrfCheckResult};
pub use shutdown::{shutdown_signal, ShutdownConfig};

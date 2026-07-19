//! Shared Axum middleware for all services.
//!
//! Provides: auth extraction, CORS, security headers, request ID, rate limiting, SSRF protection.

pub mod auth;
pub mod cors;
pub mod headers;
pub mod request_id;
pub mod rate_limit;
pub mod ssrf;

pub use auth::{AuthPrincipal, AuthMethod, JwtAuthLayer, ApiKeyAuthLayer, client_fingerprint};
pub use cors::cors_layer;
pub use headers::SecurityHeadersLayer;
pub use request_id::RequestIdLayer;
pub use rate_limit::{RateLimitLayer, RateLimitLayerConfig};
pub use ssrf::{validate_url, SsrfCheckResult};

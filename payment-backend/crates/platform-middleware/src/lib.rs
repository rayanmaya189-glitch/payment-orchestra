//! Middleware crates for the API Gateway and service pipeline.
//! Includes auth, ABAC, rate limiting, CORS, request ID, metrics, etc.

pub mod auth;
pub mod abac;
pub mod rate_limit;
pub mod cors;
pub mod request_id;
pub mod api_version;
pub mod headers;
pub mod body_limit;
pub mod metrics;
pub mod idempotency;
pub mod ssrf;
pub mod shutdown;
pub mod openapi;

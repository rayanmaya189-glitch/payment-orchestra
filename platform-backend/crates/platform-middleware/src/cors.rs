//! CORS middleware per SRS CORS-001.
//!
//! - Only explicitly whitelisted origins (operator-configured dashboard domains).
//! - Default: no cross-origin requests allowed.
//! - Preflight cache: 24 hours (86400s).

use axum::http::{HeaderValue, Method, header::HeaderName};
use tower_http::cors::{CorsLayer, AllowOrigin, AllowMethods, AllowHeaders};

use platform_config::CorsConfig;

/// Build a CORS layer from configuration.
///
/// Origins are comma-separated in config (e.g., "http://localhost:3000,https://app.example.com").
pub fn cors_layer(config: &CorsConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .allowed_origins
        .split(',')
        .filter_map(|o| {
            let trimmed = o.trim();
            if trimmed.is_empty() {
                None
            } else {
                HeaderValue::from_str(trimmed).ok()
            }
        })
        .collect();

    let max_age = std::time::Duration::from_secs(config.max_age_secs);

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-idempotency-key"),
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-csrf-token"),
        ]))
        .allow_credentials(true)
        .max_age(max_age)
}

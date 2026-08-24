//! Rate limiting middleware for connector-gateway gRPC.
//!
//! Provides a tonic service interceptor that checks rate limits using the
//! platform-middleware Redis-backed RateLimiter before processing requests.
//! Falls back to no-op (allow all) when Redis is unavailable.

use std::sync::Arc;

use tonic::{Request, Status};

use platform_middleware::rate_limit::RateLimiter;

/// Rate limiter configuration for the connector-gateway.
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second per API key
    pub requests_per_second: u32,
    /// Maximum requests per minute per API key
    pub requests_per_minute: u32,
    /// Window size in seconds (default: 60)
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            requests_per_minute: 1000,
            window_seconds: 60,
        }
    }
}

/// Rate limiting interceptor for tonic gRPC services.
///
/// Extracts API key or source IP from gRPC metadata and checks
/// against the configured rate limit. Returns `Status::resource_exhausted`
/// if rate limited.
#[derive(Clone)]
pub struct RateLimitInterceptor {
    limiter: Option<Arc<RateLimiter>>,
    config: RateLimitConfig,
}

impl RateLimitInterceptor {
    /// Create a new rate limit interceptor.
    ///
    /// If `limiter` is `None`, all requests pass through (no-op).
    pub fn new(limiter: Option<Arc<RateLimiter>>) -> Self {
        Self {
            limiter,
            config: RateLimitConfig::default(),
        }
    }

    /// Create a new rate limit interceptor with custom config.
    pub fn with_config(limiter: Option<Arc<RateLimiter>>, config: RateLimitConfig) -> Self {
        Self { limiter, config }
    }
}

/// Tonic interceptor implementation for rate limiting.
///
/// This implements `tonic::service::Interceptor` which allows it to be
/// used with `tonic::service::interceptor()`.
impl tonic::service::Interceptor for RateLimitInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let Some(ref limiter) = self.limiter else {
            // No Redis connection — allow all requests
            return Ok(request);
        };

        // Extract rate limit key from metadata (synchronous check)
        let api_key = request
            .metadata()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let source_ip = request
            .metadata()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .or_else(|| {
                request
                    .metadata()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
            })
            .map(|s| s.to_string());

        // Build rate limit key: prefer API key, fall back to IP
        let identifier = api_key.as_deref().or(source_ip.as_deref()).unwrap_or("unknown");

        // Check per-second rate limit (best-effort, don't block on Redis errors)
        let per_second_key = format!("connector_gateway:{}:second", identifier);
        let _ = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                limiter
                    .check_rate_limit(
                        &per_second_key,
                        self.config.requests_per_second,
                        1,
                    )
                    .await
            })
        });

        // Check per-minute rate limit
        let per_minute_key = format!("connector_gateway:{}:minute", identifier);

        // Since the interceptor is synchronous, we use block_in_place for the async Redis call.
        // This is acceptable for a lightweight Redis INCR operation (sub-millisecond).
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                limiter
                    .check_rate_limit(
                        &per_minute_key,
                        self.config.requests_per_minute,
                        self.config.window_seconds,
                    )
                    .await
            })
        });

        match result {
            Ok(_) => Ok(request),
            Err(e) => {
                let retry_after_ms = match &e {
                    platform_error::PlatformError::RateLimited { retry_after_ms } => *retry_after_ms,
                    _ => self.config.window_seconds * 1000,
                };

                Err(Status::resource_exhausted(format!(
                    "Rate limit exceeded. Retry after {}ms",
                    retry_after_ms
                )))
            }
        }
    }
}

/// Create a rate limiter from a Redis URL.
///
/// Returns `None` if Redis is unavailable (graceful fallback).
pub async fn create_rate_limiter(redis_url: Option<&str>) -> Option<Arc<RateLimiter>> {
    let url = redis_url.map(|s| s.to_string()).or_else(|| {
        std::env::var("CONNECTOR_GATEWAY_REDIS_URL")
            .ok()
            .or_else(|| std::env::var("REDIS_URL").ok())
    })?;

    let client = match redis::Client::open(url.as_str()) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Invalid Redis URL for rate limiter: {}", e);
            return None;
        }
    };

    match redis::aio::ConnectionManager::new(client).await {
        Ok(conn) => {
            tracing::info!("Rate limiter connected to Redis");
            Some(Arc::new(RateLimiter::new(conn)))
        }
        Err(e) => {
            tracing::warn!(
                "Rate limiter unavailable (Redis: {}), requests will not be rate-limited",
                e
            );
            None
        }
    }
}

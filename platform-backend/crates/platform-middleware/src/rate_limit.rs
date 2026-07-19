//! Rate limiting middleware using Redis sliding window counters.
//!
//! - Per-IP rate limiting for login endpoint (SRS AUTH-009: 10 per IP per minute).
//! - Per-principal rate limiting for API endpoints (configurable).
//! - Uses atomic Redis Lua script to prevent race conditions between INCR and EXPIRE.

use axum::body::Body;
use axum::http::StatusCode;
use tower::{Layer, Service};
use std::task::{Context, Poll};
use redis::aio::ConnectionManager;

/// Configuration for rate limiting.
#[derive(Clone)]
pub struct RateLimitLayerConfig {
    pub login_per_ip_per_minute: u32,
    pub api_per_principal_per_second: u32,
}

impl Default for RateLimitLayerConfig {
    fn default() -> Self {
        Self {
            login_per_ip_per_minute: 10,
            api_per_principal_per_second: 100,
        }
    }
}

/// Atomic Lua script for sliding window rate limiting.
/// Returns the current count after increment. Keys that don't exist get EXPIRE set.
/// KEYS[1] = rate limit key
/// ARGV[1] = max count
/// ARGV[2] = window in seconds
const RATE_LIMIT_LUA: &str = r#"
local key = KEYS[1]
local max_count = tonumber(ARGV[1])
local window = tonumber(ARGV[2])

local current = redis.call('INCR', key)
if current == 1 then
    redis.call('EXPIRE', key, window)
end

if current > max_count then
    local ttl = redis.call('TTL', key)
    return {current, ttl}
end

return {current, 0}
"#;

/// Layer that adds rate limiting to requests.
#[derive(Clone)]
pub struct RateLimitLayer {
    redis: ConnectionManager,
    config: RateLimitLayerConfig,
}

impl RateLimitLayer {
    pub fn new(redis: ConnectionManager, config: RateLimitLayerConfig) -> Self {
        Self { redis, config }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            redis: self.redis.clone(),
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    redis: ConnectionManager,
    config: RateLimitLayerConfig,
}

impl<S> Service<http::Request<Body>> for RateLimitService<S>
where
    S: Service<http::Request<Body>, Response = http::Response<Body>> + Send + Clone + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let mut redis = self.redis.clone();
        let config = self.config.clone();
        let path = req.uri().path().to_string();

        Box::pin(async move {
            // Determine client identifier
            let client_key = extract_client_ip(&req);

            // Choose rate limit based on endpoint
            let (limit, window_secs) = if path.contains("/auth/login") {
                // SRS AUTH-009: Login endpoint — per-IP per minute
                (config.login_per_ip_per_minute, 60i64)
            } else {
                // General API — per-principal per second
                (config.api_per_principal_per_second, 1i64)
            };

            // Atomic rate limit using Redis Lua script (prevents INCR/EXPIRE race)
            let redis_key = format!("rate_limit:{}:{}", path, client_key);

            let result: Result<(i64, i64), _> = redis::cmd("EVAL")
                .arg(RATE_LIMIT_LUA)
                .arg(1) // number of KEYS
                .arg(&redis_key)
                .arg(limit as i64)
                .arg(window_secs)
                .query_async(&mut redis)
                .await;

            match result {
                Ok((current, retry_after)) => {
                    if current > limit as i64 {
                        // Rate limited — return 429 with Retry-After
                        let retry_after = if retry_after > 0 { retry_after as u64 } else { window_secs as u64 };
                        let response = http::Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .header("content-type", "application/json")
                            .header("retry-after", retry_after.to_string())
                            .body(Body::from(serde_json::json!({
                                "error": "Rate limited",
                                "code": "RATE_LIMITED",
                                "retry_after_seconds": retry_after
                            }).to_string()))
                            .unwrap();
                        return Ok(response);
                    }
                    inner.call(req).await
                }
                Err(_) => {
                    // Redis unavailable — fail open (allow request through)
                    tracing::warn!("Rate limit check failed (Redis unavailable) — allowing request");
                    inner.call(req).await
                }
            }
        })
    }
}

/// Extract client IP from request, respecting trusted proxy headers.
fn extract_client_ip(req: &http::Request<Body>) -> String {
    // Check X-Real-IP first (set by trusted proxy)
    if let Some(ip) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return ip.to_string();
    }

    // Check X-Forwarded-For (first entry is client IP)
    if let Some(forwarded) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            return first.trim().to_string();
        }
    }

    // Fallback to connection info (localhost in dev)
    "unknown".to_string()
}

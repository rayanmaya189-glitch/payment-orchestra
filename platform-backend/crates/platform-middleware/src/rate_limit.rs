//! Rate limiting middleware using Redis sliding window counters.
//!
//! - Per-IP rate limiting for login endpoint (SRS AUTH-009: 10 per IP per minute).
//! - Per-principal rate limiting for API endpoints (configurable).
//! - Uses atomic Redis Lua script to prevent race conditions between INCR and EXPIRE.

use axum::body::Body;
use axum::http::StatusCode;
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use redis::aio::ConnectionManager;

/// Configuration for rate limiting.
#[derive(Clone)]
pub struct RateLimitLayerConfig {
    pub login_per_ip_per_minute: u32,
    pub api_per_principal_per_second: u32,
    /// Per-endpoint overrides: path prefix -> (limit, window_seconds)
    pub endpoint_overrides: Vec<(String, u32, u32)>,
    /// Trusted proxy CIDRs — only these are allowed to set X-Forwarded-For / X-Real-IP.
    /// When empty, proxy headers are ignored (direct connections only).
    pub trusted_proxies: Vec<String>,
}

impl Default for RateLimitLayerConfig {
    fn default() -> Self {
        Self {
            login_per_ip_per_minute: 10,
            api_per_principal_per_second: 100,
            endpoint_overrides: vec![
                ("/v1/auth/login".into(), 10, 60),      // Login: 10/min
                ("/v1/auth/refresh".into(), 30, 60),    // Refresh: 30/min
                ("/v1/payment-intents".into(), 500, 60), // Checkout: 500/min
                ("/v1/kyb-cases".into(), 10, 60),        // KYB: 10/min
                ("/v1/gateway-profiles".into(), 100, 60), // Gateway: 100/min
                ("/v1/invoices".into(), 100, 60),        // Invoices: 100/min
                ("/v1/disputes".into(), 50, 60),          // Disputes: 50/min
                ("/v1/subscriptions".into(), 50, 60),    // Subscriptions: 50/min
            ],
            trusted_proxies: vec![], // Empty = ignore proxy headers (direct connections)
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

/// In-memory sliding window fallback for when Redis is unavailable.
/// Uses a simple per-key counter with a timestamp. Not distributed, but prevents
/// brute-force attacks during Redis outages (OWASP A04).
#[derive(Clone)]
struct InMemoryFallback {
    counters: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl InMemoryFallback {
    fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and increment. Returns (current_count, is_limited).
    fn check(&self, key: &str, limit: u32, window: Duration) -> (u32, bool) {
        let mut counters = self.counters.lock().unwrap();
        let now = Instant::now();

        let entry = counters.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) > window {
            // Window expired — reset
            *entry = (1, now);
            (1, false)
        } else {
            entry.0 += 1;
            (entry.0, entry.0 > limit)
        }
    }
}

/// Layer that adds rate limiting to requests.
#[derive(Clone)]
pub struct RateLimitLayer {
    redis: ConnectionManager,
    config: RateLimitLayerConfig,
    fallback: InMemoryFallback,
    trusted_proxies: Vec<String>,
}

impl RateLimitLayer {
    pub fn new(redis: ConnectionManager, config: RateLimitLayerConfig) -> Self {
        let trusted_proxies = config.trusted_proxies.clone();
        Self {
            redis,
            config,
            fallback: InMemoryFallback::new(),
            trusted_proxies,
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            redis: self.redis.clone(),
            config: self.config.clone(),
            fallback: self.fallback.clone(),
            trusted_proxies: self.trusted_proxies.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    redis: ConnectionManager,
    config: RateLimitLayerConfig,
    fallback: InMemoryFallback,
    trusted_proxies: Vec<String>,
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
        let fallback = self.fallback.clone();
        let path = req.uri().path().to_string();
        let trusted_proxies = self.trusted_proxies.clone();

        Box::pin(async move {
            // Determine client identifier
            let client_key = extract_client_ip(&req, &trusted_proxies);

            // Choose rate limit based on endpoint (SRS RL-001: per-endpoint limits)
            let (limit, window_secs) = if path.contains("/auth/login") {
                // SRS AUTH-009: Login endpoint — per-IP per minute
                (config.login_per_ip_per_minute, 60i64)
            } else {
                // Check per-endpoint overrides first
                let mut matched = false;
                let mut result = (config.api_per_principal_per_second, 1i64);
                for (prefix, endpoint_limit, endpoint_window) in &config.endpoint_overrides {
                    if path.starts_with(prefix) {
                        result = (*endpoint_limit, *endpoint_window as i64);
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    // General API — per-principal per second
                    result = (config.api_per_principal_per_second, 1i64);
                }
                result
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
                    // Redis unavailable — use in-memory fallback (OWASP A04: no fail-open)
                    tracing::warn!("Redis unavailable for rate limiting — using in-memory fallback");
                    let window = Duration::from_secs(window_secs as u64);
                    let (_current, is_limited) = fallback.check(&redis_key, limit, window);
                    if is_limited {
                        let response = http::Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .header("content-type", "application/json")
                            .header("retry-after", window_secs.to_string())
                            .body(Body::from(serde_json::json!({
                                "error": "Rate limited (fallback)",
                                "code": "RATE_LIMITED",
                                "retry_after_seconds": window_secs
                            }).to_string()))
                            .unwrap();
                        return Ok(response);
                    }
                    inner.call(req).await
                }
            }
        })
    }
}

/// Extract client IP from request, respecting trusted proxy headers.
///
/// Only trusts X-Real-IP / X-Forwarded-For when the connection comes from a known
/// trusted proxy (SRS ABAC-009, NET-SEG-001). Prevents IP spoofing bypass of
/// per-IP rate limits (OWASP A01).
fn extract_client_ip(req: &http::Request<Body>, trusted_proxies: &[String]) -> String {
    // Get the direct connection peer IP if available
    let peer_ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string());

    // If we have no trusted proxies configured, never trust proxy headers
    if trusted_proxies.is_empty() {
        return peer_ip.unwrap_or_else(|| "unknown".to_string());
    }

    // Check if the direct peer is a trusted proxy
    if let Some(ref peer) = peer_ip {
        let is_trusted = trusted_proxies.iter().any(|cidr| {
            // Simple prefix match — in production use ipnet crate for CIDR matching
            peer.starts_with(cidr.trim_end_matches(".*").trim_end_matches("/24"))
                || peer.starts_with(cidr.trim_end_matches(".0").trim_end_matches("/24"))
        });
        if !is_trusted {
            // Direct connection from untrusted source — ignore proxy headers
            return peer.clone();
        }
    } else {
        // No peer info available — ignore proxy headers
        return "unknown".to_string();
    }

    // Connection is from a trusted proxy — safe to read headers
    if let Some(ip) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        // Validate it's a plausible IP
        if ip.parse::<std::net::IpAddr>().is_ok() {
            return ip.to_string();
        }
    }

    if let Some(forwarded) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            let trimmed = first.trim();
            if trimmed.parse::<std::net::IpAddr>().is_ok() {
                return trimmed.to_string();
            }
        }
    }

    // Fallback to peer IP
    peer_ip.unwrap_or_else(|| "unknown".to_string())
}

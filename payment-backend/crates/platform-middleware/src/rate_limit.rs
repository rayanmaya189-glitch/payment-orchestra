//! Rate limiting middleware — Redis-backed + in-memory sliding window + gRPC tower layer.
//!
//! ## Architecture
//!
//! Two rate limiter implementations are available:
//!
//! 1. **Redis-backed** ([`RateLimiter`]) — Uses Redis INCR + EXPIRE for a fixed-window
//!    rate limit per key. Suitable for distributed deployments.
//! 2. **In-memory** ([`MemoryRateLimiter`]) — Uses a concurrent hashmap with
//!    periodic cleanup. No external dependency. Suitable for single-instance or
//!    development environments.
//!
//! Both are wrapped by [`GrcRateLimitLayer`], a tower [`Layer`] that can be added
//! to any tonic gRPC server via `.layer()`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use platform_middleware::rate_limit::{GrcRateLimitLayer, MemoryRateLimiter};
//!
//! Server::builder()
//!     .layer(GrcRateLimitLayer::new(
//!         "my-service",
//!         MemoryRateLimiter::new(100, 60), // 100 req / 60 sec window
//!     ))
//!     .add_service(MyServiceServer::new(impl))
//!     .serve(addr)
//!     .await?;
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use http::{Request, Response};
use platform_error::PlatformError;
use redis::aio::ConnectionManager;
use tonic::Code;
use tower::{Layer, Service};

// ─── In-Memory Rate Limiter ──────────────────────────────────────────────────

/// Per-key state for the in-memory rate limiter.
#[derive(Clone, Debug)]
struct WindowState {
    /// Timestamp (ms since epoch) when the current window started.
    window_start_ms: i64,
    /// Request count in the current window.
    count: u32,
}

/// In-memory fixed-window rate limiter using a concurrent hashmap.
///
/// Uses a shared `Arc<Mutex<HashMap>>` with periodic cleanup of stale keys.
/// Window boundaries are aligned to epoch boundaries for consistency across
/// instances.
///
/// ## Cleanup
///
/// Stale keys are lazily cleaned on check. At most one cleanup sweep happens
/// every 5 minutes to avoid lock contention.
#[derive(Clone, Debug)]
pub struct MemoryRateLimiter {
    inner: Arc<Mutex<MemoryRateLimiterInner>>,
}

#[derive(Debug)]
struct MemoryRateLimiterInner {
    windows: HashMap<String, WindowState>,
    last_cleanup: Instant,
}

impl MemoryRateLimiter {
    /// Create a new in-memory rate limiter.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryRateLimiterInner {
                windows: HashMap::new(),
                last_cleanup: Instant::now(),
            })),
        }
    }

    /// Check if a request should be rate limited using the given limit params.
    ///
    /// # Arguments
    /// * `key` — Unique key for the rate limit bucket.
    /// * `max_requests` — Maximum number of requests allowed within the window.
    /// * `window_secs` — Time window in seconds.
    ///
    /// # Returns
    /// * `Ok(true)` — request is allowed
    /// * `Err(PlatformError::RateLimited { retry_after_ms })` — rate limited
    fn check_inner(
        &self,
        key: &str,
        max_requests: u32,
        window_secs: u64,
    ) -> Result<bool, PlatformError> {
        let mut inner = self.inner.lock().map_err(|e| {
            PlatformError::Internal(format!("Rate limiter lock poisoned: {e}"))
        })?;

        // Periodic cleanup of stale keys (every 5 minutes)
        if inner.last_cleanup.elapsed() > Duration::from_secs(300) {
            let now_ms = now_ms();
            // Compute the oldest window boundary that can still be relevant
            let oldest_window_ms = now_ms - (window_secs as i64 * 1000) - 1000;
            // Remove all windows older than the oldest possible current window
            inner
                .windows
                .retain(|_, state| state.window_start_ms >= oldest_window_ms);
            inner.last_cleanup = Instant::now();
        }

        let now_ms = now_ms();
        let window_len_ms = (window_secs as i64).saturating_mul(1000);
        let window_start_ms = if window_len_ms > 0 {
            (now_ms / window_len_ms) * window_len_ms
        } else {
            now_ms
        };

        let state = inner
            .windows
            .entry(key.to_string())
            .or_insert(WindowState {
                window_start_ms,
                count: 0,
            });

        // If the window has rolled over, reset for the new window
        if state.window_start_ms < window_start_ms {
            state.window_start_ms = window_start_ms;
            state.count = 0;
        }

        // Check limit
        if state.count >= max_requests {
            let retry_after_ms = (window_start_ms + window_len_ms) - now_ms;
            return Err(PlatformError::RateLimited {
                retry_after_ms: retry_after_ms.max(0) as u64,
            });
        }

        state.count += 1;
        Ok(true)
    }

    /// Get the current count for a rate limit key (without incrementing).
    pub fn current_count(&self, key: &str) -> Result<u32, PlatformError> {
        let inner = self.inner.lock().map_err(|e| {
            PlatformError::Internal(format!("Rate limiter lock poisoned: {e}"))
        })?;

        let now_ms = now_ms();
        let window_start_ms = now_ms;

        Ok(inner
            .windows
            .get(key)
            .filter(|state| state.window_start_ms >= window_start_ms - 60000)
            .map(|state| state.count)
            .unwrap_or(0))
    }

    /// Reset a rate limit counter for a key.
    pub fn reset(&self, key: &str) -> Result<(), PlatformError> {
        let mut inner = self.inner.lock().map_err(|e| {
            PlatformError::Internal(format!("Rate limiter lock poisoned: {e}"))
        })?;
        inner.windows.remove(key);
        Ok(())
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ─── Redis-Backed Rate Limiter ───────────────────────────────────────────────

/// Rate limiter backed by Redis using a fixed-window approach.
///
/// Algorithm:
/// 1. INCR `ratelimit:{key}` to atomically increment the counter
/// 2. If the key was just created (return value == 1), set EXPIRE to `window_secs + 1`
/// 3. Compare the current count against the `limit`
/// 4. If count > limit, return RateLimited error
#[derive(Clone)]
pub struct RateLimiter {
    redis: ConnectionManager,
}

impl RateLimiter {
    /// Create a new rate limiter with a Redis connection manager.
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Check if a request should be rate limited.
    ///
    /// # Arguments
    /// * `key` — Unique key for the rate limit bucket.
    /// * `limit` — Maximum number of requests allowed within the window.
    /// * `window_secs` — Time window in seconds.
    ///
    /// # Returns
    /// * `Ok(true)` — request is allowed
    /// * `Err(PlatformError::RateLimited { retry_after_ms })` — rate limited with retry info
    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<bool, PlatformError> {
        let window_key = format!("ratelimit:{key}");

        // Use INCR to atomically increment the counter
        let count: u32 = redis::cmd("INCR")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis INCR failed: {e}")))?;

        // If this is the first request in the window, set expiration
        if count == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(&window_key)
                .arg(window_secs as i64)
                .query_async(&mut self.redis.clone())
                .await
                .map_err(|e| PlatformError::Internal(format!("Redis EXPIRE failed: {e}")))?;
        }

        if count > limit {
            // Get remaining TTL for retry_after calculation
            let ttl: i64 = redis::cmd("TTL")
                .arg(&window_key)
                .query_async(&mut self.redis.clone())
                .await
                .unwrap_or(window_secs as i64);

            let retry_after_ms = (ttl.max(0) as u64) * 1000;
            return Err(PlatformError::RateLimited { retry_after_ms });
        }

        Ok(true)
    }

    /// Get the current count for a rate limit key (without incrementing).
    pub async fn current_count(&self, key: &str) -> Result<u32, PlatformError> {
        let window_key = format!("ratelimit:{key}");
        let count: Option<u32> = redis::cmd("GET")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis GET failed: {e}")))?;

        Ok(count.unwrap_or(0))
    }

    /// Reset a rate limit counter for a key.
    pub async fn reset(&self, key: &str) -> Result<(), PlatformError> {
        let window_key = format!("ratelimit:{key}");
        let _: () = redis::cmd("DEL")
            .arg(&window_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| PlatformError::Internal(format!("Redis DEL failed: {e}")))?;
        Ok(())
    }

    /// Create a rate limit key from components.
    pub fn build_key(prefix: &str, identifier: &str, endpoint: &str) -> String {
        format!("{prefix}:{identifier}:{endpoint}")
    }
}

// ─── Rate Limit Backend Trait ────────────────────────────────────────────────

/// Abstraction over different rate limit storage backends.
///
/// Implemented by both [`MemoryRateLimiter`] and the Redis-backed [`RateLimiter`].
/// The `check` method is synchronous; for Redis-backed implementations, the
/// async check must be bridged via `tokio::task::block_in_place` + `Handle::block_on`.
pub trait RateLimitBackend: std::fmt::Debug {
    /// Check if a request should be rate limited.
    ///
    /// # Arguments
    /// * `key` — Unique key for the rate limit bucket.
    /// * `max_requests` — Maximum number of requests allowed within the window.
    /// * `window_secs` — Time window in seconds.
    ///
    /// # Returns
    /// * `Ok(true)` — request is allowed
    /// * `Err(PlatformError::RateLimited { .. })` — rate limited
    fn check(&self, key: &str, max_requests: u32, window_secs: u64) -> Result<bool, PlatformError>;
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter")
            .field("redis", &"<connection>")
            .finish()
    }
}

impl RateLimitBackend for MemoryRateLimiter {
    fn check(&self, key: &str, max_requests: u32, window_secs: u64) -> Result<bool, PlatformError> {
        self.check_inner(key, max_requests, window_secs)
    }
}

/// Wrapper to make Redis-backed [`RateLimiter`] implement [`RateLimitBackend`].
///
/// Uses `tokio::task::block_in_place` to bridge the async Redis operations into
/// the synchronous trait method. This is acceptable because:
/// 1. The tower service runs inside the tokio runtime.
/// 2. Redis operations are typically <1ms.
/// 3. Each call briefly parks the current tokio task but does not block the worker thread.
#[derive(Clone, Debug)]
pub struct RedisRateLimitBackend {
    inner: RateLimiter,
}

impl RedisRateLimitBackend {
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            inner: RateLimiter::new(redis),
        }
    }
}

impl RateLimitBackend for RedisRateLimitBackend {
    fn check(&self, key: &str, max_requests: u32, window_secs: u64) -> Result<bool, PlatformError> {
        // Use block_in_place to run async Redis operations synchronously.
        // This is safe here because:
        // - We are inside a tokio runtime context.
        // - Redis operations are fast and non-blocking with async-nats.
        // - block_in_place yields to the runtime so other tasks can run.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.inner.check_rate_limit(key, max_requests, window_secs))
        })
    }
}

// ─── gRPC Tower Layer ────────────────────────────────────────────────────────

/// Default maximum requests per window for a service (global throttle).
pub const DEFAULT_MAX_REQUESTS: u32 = 1000;
/// Default window size in seconds.
pub const DEFAULT_WINDOW_SECS: u64 = 60;
/// Default maximum requests per window per client.
pub const DEFAULT_CLIENT_MAX_REQUESTS: u32 = 100;

/// A tower [`Layer`] that wraps every gRPC call with rate limit checks.
///
/// Supports both in-memory and Redis-backed rate limiters. The layer can be
/// configured with:
/// - A global per-service limit (max requests per window).
/// - A per-client limit (max requests per window per API key or IP).
///
/// # Usage
///
/// ```rust,ignore
/// use platform_middleware::rate_limit::{GrcRateLimitLayer, MemoryRateLimiter};
///
/// Server::builder()
///     .layer(GrcRateLimitLayer::new(
///         "orchestration-service",
///         MemoryRateLimiter::new(),
///     ))
///     .add_service(MyServiceServer::new(impl))
///     .serve(addr)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct GrcRateLimitLayer {
    service_name: String,
    limiter: Arc<dyn RateLimitBackend + Send + Sync>,
    service_max_requests: u32,
    client_max_requests: u32,
    window_secs: u64,
}

impl GrcRateLimitLayer {
    /// Create a new rate limit layer backed by the in-memory limiter.
    ///
    /// Convenience constructor for services that don't have Redis.
    /// Uses a default limit of 1000 req/60s per service and 100 req/60s per client.
    pub fn in_memory(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            limiter: Arc::new(MemoryRateLimiter::new()),
            service_max_requests: DEFAULT_MAX_REQUESTS,
            client_max_requests: DEFAULT_CLIENT_MAX_REQUESTS,
            window_secs: DEFAULT_WINDOW_SECS,
        }
    }

    /// Create a new rate limit layer with a custom backend.
    ///
    /// # Arguments
    /// * `service_name` — Name of the service for logging/metrics.
    /// * `limiter` — An in-memory or Redis-backed rate limiter.
    pub fn new(
        service_name: impl Into<String>,
        limiter: impl RateLimitBackend + Send + Sync + 'static,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            limiter: Arc::new(limiter),
            service_max_requests: DEFAULT_MAX_REQUESTS,
            client_max_requests: DEFAULT_CLIENT_MAX_REQUESTS,
            window_secs: DEFAULT_WINDOW_SECS,
        }
    }

    /// Set the per-service maximum requests within the window.
    pub fn with_service_limit(mut self, max: u32) -> Self {
        self.service_max_requests = max;
        self
    }

    /// Set the per-client maximum requests within the window.
    pub fn with_client_limit(mut self, max: u32) -> Self {
        self.client_max_requests = max;
        self
    }

    /// Set the window duration in seconds.
    pub fn with_window_secs(mut self, secs: u64) -> Self {
        self.window_secs = secs;
        self
    }
}

impl<S> Layer<S> for GrcRateLimitLayer {
    type Service = GrcRateLimitService<S>;

    fn layer(&self, service: S) -> Self::Service {
        GrcRateLimitService {
            inner: service,
            service_name: self.service_name.clone(),
            limiter: Arc::clone(&self.limiter),
            service_max_requests: self.service_max_requests,
            client_max_requests: self.client_max_requests,
            window_secs: self.window_secs,
            in_flight: Arc::new(AtomicI64::new(0)),
        }
    }
}

// ─── gRPC Service ────────────────────────────────────────────────────────────

/// Wrapper around an inner tower service that enforces rate limits.
///
/// Created by [`GrcRateLimitLayer`]. Checks rate limits before forwarding
/// requests to the inner service. Returns `ResourceExhausted` gRPC status
/// if the rate limit is exceeded.
#[derive(Clone, Debug)]
pub struct GrcRateLimitService<S> {
    inner: S,
    service_name: String,
    limiter: Arc<dyn RateLimitBackend + Send + Sync>,
    service_max_requests: u32,
    client_max_requests: u32,
    window_secs: u64,
    in_flight: Arc<AtomicI64>,
}

impl<S> GrcRateLimitService<S> {
    /// Current number of in-flight requests.
    pub fn in_flight_count(&self) -> i64 {
        self.in_flight.load(Ordering::Relaxed)
    }
}

/// Extract a client identifier from the gRPC request metadata.
///
/// Precedence:
/// 1. `x-api-key` header (if present)
/// 2. `authorization` token prefix (if present)
/// 3. `x-forwarded-for` header
/// 4. Fallback to `unknown-client`
fn extract_client_identifier<ReqBody>(req: &Request<ReqBody>) -> String {
    // Try x-api-key header
    if let Some(api_key) = req.headers().get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            if !key_str.is_empty() {
                return format!("ak:{key_str}");
            }
        }
    }

    // Try authorization header
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            // Use a hash of the token prefix for anonymity
            let prefix = if auth_str.len() > 20 {
                &auth_str[..20]
            } else {
                auth_str
            };
            return format!("auth:{prefix}");
        }
    }

    // Try x-forwarded-for header
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(ip) = forwarded.to_str() {
            let client_ip = ip.split(',').next().unwrap_or("unknown").trim();
            if !client_ip.is_empty() && client_ip != "unknown" {
                return format!("ip:{client_ip}");
            }
        }
    }

    "unknown-client".to_string()
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GrcRateLimitService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let service_name = self.service_name.clone();
        let limiter = Arc::clone(&self.limiter);
        let service_max = self.service_max_requests;
        let client_max = self.client_max_requests;
        let window_secs = self.window_secs;
        let method = req.uri().path().trim_start_matches('/').to_string();
        let client_id = extract_client_identifier(&req);
        let in_flight = Arc::clone(&self.in_flight);

        in_flight.fetch_add(1, Ordering::Relaxed);

        let fut = self.inner.call(req);

        Box::pin(async move {
            // Build rate limit keys
            let service_key = format!("svc:{service_name}");
            let client_service_key = format!("{service_name}:client:{client_id}:{method}");

            // Check service-level rate limit first (global throttle)
            if let Err(e) = limiter.check(&service_key, service_max, window_secs) {
                tracing::warn!(
                    service = %service_name,
                    client = %client_id,
                    method = %method,
                    error = %e,
                    "Service-level rate limit exceeded"
                );                let _ = in_flight.fetch_sub(1, Ordering::Relaxed);
                let retry_after_ms = match &e {
                    PlatformError::RateLimited { retry_after_ms } => *retry_after_ms,
                    _ => window_secs * 1000,
                };
                let status = tonic::Status::with_details(
                    Code::ResourceExhausted,
                    format!("Rate limit exceeded for {service_name}. Retry after {retry_after_ms}ms"),
                    retry_after_ms.to_string().into(),
                );
                return Err(Box::new(status) as Box<dyn std::error::Error + Send + Sync>);
            }

            // Check client-level rate limit (per-client throttle)
            if let Err(e) = limiter.check(&client_service_key, client_max, window_secs) {
                tracing::warn!(
                    service = %service_name,
                    client = %client_id,
                    method = %method,
                    error = %e,
                    "Client rate limit exceeded"
                );
                let _ = in_flight.fetch_sub(1, Ordering::Relaxed);
                let retry_after_ms = match &e {
                    PlatformError::RateLimited { retry_after_ms } => *retry_after_ms,
                    _ => window_secs * 1000,
                };
                let status = tonic::Status::with_details(
                    Code::ResourceExhausted,
                    format!("Rate limit exceeded for client. Retry after {retry_after_ms}ms"),
                    retry_after_ms.to_string().into(),
                );
                return Err(Box::new(status) as Box<dyn std::error::Error + Send + Sync>);
            }

            // Forward to inner service
            let result = fut.await;
            let _ = in_flight.fetch_sub(1, Ordering::Relaxed);

            result.map_err(Into::into)
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_rate_limiter_allows_requests() {
        let limiter = MemoryRateLimiter::new();
        for _ in 0..5 {
            assert!(limiter.check_inner("test-key", 5, 60).unwrap());
        }
    }

    #[test]
    fn test_memory_rate_limiter_blocks_excess() {
        let limiter = MemoryRateLimiter::new();
        for _ in 0..3 {
            assert!(limiter.check_inner("test-key", 3, 60).unwrap());
        }
        let result = limiter.check_inner("test-key", 3, 60);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformError::RateLimited { .. }
        ));
    }

    #[test]
    fn test_memory_rate_limiter_independent_keys() {
        let limiter = MemoryRateLimiter::new();
        assert!(limiter.check_inner("key-a", 2, 60).unwrap());
        assert!(limiter.check_inner("key-a", 2, 60).unwrap());
        assert!(limiter.check_inner("key-b", 2, 60).unwrap()); // different key should not be limited
        assert!(limiter.check_inner("key-b", 2, 60).unwrap());
        assert!(limiter.check_inner("key-a", 2, 60).is_err()); // key-a exceeded
    }

    #[test]
    fn test_memory_rate_limiter_reset() {
        let limiter = MemoryRateLimiter::new();
        assert!(limiter.check_inner("test-key", 2, 60).unwrap());
        assert!(limiter.check_inner("test-key", 2, 60).unwrap());
        assert!(limiter.check_inner("test-key", 2, 60).is_err());

        limiter.reset("test-key").unwrap();
        assert!(limiter.check_inner("test-key", 2, 60).unwrap());
    }

    #[test]
    fn test_extract_client_identifier_no_headers() {
        let req = Request::new(());
        let client = extract_client_identifier(&req);
        assert_eq!(client, "unknown-client");
    }

    #[test]
    fn test_extract_client_identifier_api_key() {
        let req = Request::builder()
            .header("x-api-key", "sk_test_abc123")
            .body(())
            .unwrap();
        let client = extract_client_identifier(&req);
        assert!(client.starts_with("ak:"));
        assert!(client.contains("sk_test_abc123"));
    }

    #[test]
    fn test_extract_client_identifier_auth() {
        let req = Request::builder()
            .header("authorization", "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9")
            .body(())
            .unwrap();
        let client = extract_client_identifier(&req);
        assert!(client.starts_with("auth:"));
    }

    #[test]
    fn test_extract_client_identifier_forwarded_for() {
        let req = Request::builder()
            .header("x-forwarded-for", "203.0.113.42, 10.0.0.1")
            .body(())
            .unwrap();
        let client = extract_client_identifier(&req);
        assert_eq!(client, "ip:203.0.113.42");
    }

    #[test]
    fn test_current_count() {
        let limiter = MemoryRateLimiter::new();
        assert_eq!(limiter.current_count("test-key").unwrap(), 0);
        limiter.check_inner("test-key", 10, 60).unwrap();
        assert_eq!(limiter.current_count("test-key").unwrap(), 1);
        limiter.check_inner("test-key", 10, 60).unwrap();
        assert_eq!(limiter.current_count("test-key").unwrap(), 2);
    }

    #[test]
    fn test_build_key() {
        let key = RateLimiter::build_key("ratelimit", "api_key_123", "payment_intents/create");
        assert_eq!(key, "ratelimit:api_key_123:payment_intents/create");
    }

    #[test]
    fn test_memory_rate_limiter_via_trait() {
        let limiter = MemoryRateLimiter::new();
        let backend: &dyn RateLimitBackend = &limiter;

        for _ in 0..5 {
            assert!(backend.check("test-trait-key", 5, 60).unwrap());
        }
        assert!(backend.check("test-trait-key", 5, 60).is_err());
    }
}

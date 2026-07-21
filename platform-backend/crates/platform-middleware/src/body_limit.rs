//! Request body size limit middleware (OWASP A04: Insecure Design).
//!
//! Prevents memory exhaustion from oversized request payloads.
//! SRS REQ-001: Standard 1MB, document upload 10MB, payment creation 100KB.

use axum::body::Body;
use axum::http::StatusCode;
use tower::{Layer, Service};
use std::task::{Context, Poll};

/// Configuration for request body size limits.
#[derive(Clone)]
pub struct BodyLimitConfig {
    /// Default max body size in bytes (1MB)
    pub default_max_bytes: usize,
    /// Per-path overrides: (path prefix, max bytes)
    pub path_overrides: Vec<(String, usize)>,
}

impl Default for BodyLimitConfig {
    fn default() -> Self {
        Self {
            default_max_bytes: 1024 * 1024, // 1MB (SRS REQ-001)
            path_overrides: vec![
                ("/v1/documents".into(), 10 * 1024 * 1024),  // 10MB for document upload
                ("/v1/payment-intents".into(), 100 * 1024),  // 100KB for payment creation
                ("/v1/invoices".into(), 512 * 1024),          // 512KB for invoice creation
                ("/pay/".into(), 256 * 1024),                 // 256KB for hosted checkout
            ],
        }
    }
}

/// Layer that rejects requests with oversized bodies.
#[derive(Clone)]
pub struct BodyLimitLayer {
    config: BodyLimitConfig,
}

impl BodyLimitLayer {
    pub fn new(config: BodyLimitConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for BodyLimitLayer {
    type Service = BodyLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodyLimitService {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BodyLimitService<S> {
    inner: S,
    config: BodyLimitConfig,
}

impl<S> Service<http::Request<Body>> for BodyLimitService<S>
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
        let config = self.config.clone();
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        Box::pin(async move {
            // Skip body limit for methods that don't have a body
            if method == http::Method::GET || method == http::Method::HEAD || method == http::Method::OPTIONS {
                return inner.call(req).await;
            }

            // Determine max size for this path
            let max_bytes = config
                .path_overrides
                .iter()
                .find(|(prefix, _)| path.starts_with(prefix))
                .map(|(_, size)| *size)
                .unwrap_or(config.default_max_bytes);

            // Check Content-Length header first (fast path)
            if let Some(content_length) = req.headers().get("content-length") {
                if let Ok(len_str) = content_length.to_str() {
                    if let Ok(len) = len_str.parse::<usize>() {
                        if len > max_bytes {
                            let response = http::Response::builder()
                                .status(StatusCode::PAYLOAD_TOO_LARGE)
                                .header("content-type", "application/json")
                                .body(Body::from(serde_json::json!({
                                    "error": "Request body too large",
                                    "code": "PAYLOAD_TOO_LARGE",
                                    "max_bytes": max_bytes,
                                }).to_string()))
                                .unwrap();
                            return Ok(response);
                        }
                    }
                }
            }

            // For streaming bodies, we can't check size upfront — let it through
            // but the axum Json extractor will enforce limits on parsed bodies.
            // This is a defense-in-depth measure for Content-Length.
            inner.call(req).await
        })
    }
}

//! Security headers middleware per SRS HDR-001.
//!
//! Adds: Strict-Transport-Security, X-Content-Type-Options, Content-Security-Policy,
//! X-Frame-Options, Referrer-Policy, Permissions-Policy.

use axum::body::Body;
use tower::{Layer, Service};
use std::task::{Context, Poll};

/// Layer that adds security headers to all responses.
#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService { inner }
    }
}

#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
}

impl<S> Service<http::Request<Body>> for SecurityHeadersService<S>
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

        Box::pin(async move {
            let mut response = inner.call(req).await?;

            let headers = response.headers_mut();

            // SRS HDR-001: Strict-Transport-Security
            headers.insert(
                "strict-transport-security",
                "max-age=31536000; includeSubDomains; preload".parse().unwrap(),
            );

            // SRS HDR-001: Prevent MIME type sniffing
            headers.insert("x-content-type-options", "nosniff".parse().unwrap());

            // SRS HDR-001: Prevent framing (clickjacking)
            headers.insert("x-frame-options", "DENY".parse().unwrap());

            // SRS HDR-001: Content Security Policy
            headers.insert(
                "content-security-policy",
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".parse().unwrap(),
            );

            // SRS HDR-001: Referrer Policy
            headers.insert(
                "referrer-policy",
                "strict-origin-when-cross-origin".parse().unwrap(),
            );

            // SRS HDR-001: Permissions Policy (disable unnecessary browser features)
            headers.insert(
                "permissions-policy",
                "camera=(), microphone=(), geolocation=(), payment=()".parse().unwrap(),
            );

            // Disable XSS Protection (modern browsers use CSP instead)
            headers.insert("x-xss-protection", "0".parse().unwrap());

            Ok(response)
        })
    }
}

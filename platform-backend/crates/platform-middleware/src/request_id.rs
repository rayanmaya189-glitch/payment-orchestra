//! Request ID middleware per SRS REQ-003.
//!
//! - Generates UUIDv7-based `request_id` for every inbound request.
//! - Propagates in response header `X-Request-ID`.
//! - Available as extension for downstream services.

use axum::body::Body;
use tower::{Layer, Service};
use std::task::{Context, Poll};
use uuid::Uuid;

/// Layer that generates a unique request ID for every inbound request.
#[derive(Clone)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService { inner }
    }
}

#[derive(Clone)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<S> Service<http::Request<Body>> for RequestIdService<S>
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

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Use existing X-Request-ID header if present, otherwise generate new UUIDv7
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| Uuid::parse_str(v).ok())
                .unwrap_or_else(Uuid::now_v7);

            let request_id_str = request_id.to_string();

            // Store in extensions for downstream use
            req.extensions_mut().insert(request_id);

            let mut response = inner.call(req).await?;

            // Add to response headers
            if let Ok(val) = request_id_str.parse() {
                response.headers_mut().insert("x-request-id", val);
            }

            Ok(response)
        })
    }
}

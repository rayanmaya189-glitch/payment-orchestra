//! Request ID and Correlation ID middleware for distributed tracing.
//!
//! Provides:
//! - Unique request ID generation (UUIDv7 for time-ordered IDs)
//! - Correlation ID propagation across service boundaries
//! - gRPC metadata propagation via `x-correlation-id` header
//! - HTTP header propagation via `X-Correlation-ID` header

use http::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

/// Generate a unique request ID using UUIDv7 (time-ordered).
pub fn generate_request_id() -> String {
    Uuid::now_v7().to_string()
}

/// Generate a correlation ID (alias for request ID).
pub fn generate_correlation_id() -> String {
    generate_request_id()
}

/// Header name for correlation ID propagation.
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// Header name for request ID.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

// ─── Correlation ID Extension ────────────────────────────────────────────────

/// Extension key for correlation ID in request/response extensions.
#[derive(Debug, Clone)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Extract correlation ID from request extensions.
    pub fn from_request<B>(req: &Request<B>) -> Option<String> {
        req.extensions()
            .get::<CorrelationId>()
            .map(|c| c.0.clone())
    }

    /// Extract correlation ID from response extensions.
    pub fn from_response<B>(res: &Response<B>) -> Option<String> {
        res.extensions()
            .get::<CorrelationId>()
            .map(|c| c.0.clone())
    }
}

// ─── Correlation ID Layer ────────────────────────────────────────────────────

/// Layer that extracts or generates a correlation ID for each request
/// and adds it to response headers.
#[derive(Clone)]
pub struct CorrelationIdLayer;

impl<S> Layer<S> for CorrelationIdLayer {
    type Service = CorrelationIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorrelationIdService { inner }
    }
}

// ─── Correlation ID Service ──────────────────────────────────────────────────

/// Service that extracts correlation ID from request headers, stores it
/// in request extensions, and adds it to response headers.
#[derive(Clone)]
pub struct CorrelationIdService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CorrelationIdService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Extract or generate correlation ID
        let correlation_id = req
            .headers()
            .get(CORRELATION_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(generate_correlation_id);

        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(generate_request_id);

        // Add correlation ID to request extensions
        let mut req = req;
        req.extensions_mut()
            .insert(CorrelationId(correlation_id.clone()));
        req.extensions_mut()
            .insert(CorrelationId(request_id.clone()));

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(req).await?;

            // Add correlation ID to response headers
            response.headers_mut().insert(
                CORRELATION_ID_HEADER,
                correlation_id
                    .parse()
                    .unwrap_or(http::HeaderValue::from_static("unknown")),
            );
            response.headers_mut().insert(
                REQUEST_ID_HEADER,
                request_id
                    .parse()
                    .unwrap_or(http::HeaderValue::from_static("unknown")),
            );

            Ok(response)
        })
    }
}

// ─── gRPC Metadata Propagation ───────────────────────────────────────────────

/// Propagate correlation ID through gRPC metadata.
///
/// For tonic-based services, this should be called when building
/// requests to downstream services.
pub fn propagate_correlation_id(
    request: &mut tonic::Request<impl Sized>,
    correlation_id: &str,
) {
    if let Ok(value) = tonic::metadata::MetadataValue::from_str(correlation_id) {
        request
            .metadata_mut()
            .insert(CORRELATION_ID_HEADER, value);
    }
}

/// Extract correlation ID from gRPC metadata.
pub fn extract_correlation_id_from_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Option<String> {
    metadata
        .get(CORRELATION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id = generate_request_id();
        assert!(!id.is_empty());
        // UUIDv7 format: 8-4-4-4-12
        assert_eq!(id.matches('-').count(), 4);
    }

    #[test]
    fn test_generate_correlation_id() {
        let id = generate_correlation_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_correlation_id_from_request() {
        let req = Request::new(());
        assert!(CorrelationId::from_request(&req).is_none());

        let mut req = Request::new(());
        req.extensions_mut()
            .insert(CorrelationId("test-123".into()));
        assert_eq!(
            CorrelationId::from_request(&req),
            Some("test-123".into())
        );
    }

    #[test]
    fn test_correlation_id_from_response() {
        let res = Response::new(());
        assert!(CorrelationId::from_response(&res).is_none());

        let mut res = Response::new(());
        res.extensions_mut()
            .insert(CorrelationId("resp-456".into()));
        assert_eq!(
            CorrelationId::from_response(&res),
            Some("resp-456".into())
        );
    }
}

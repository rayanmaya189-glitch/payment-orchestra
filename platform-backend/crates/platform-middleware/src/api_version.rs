//! API versioning middleware (SRS API-001: URL path versioning /v1/...).
//!
//! Enforces that all API requests use the /v1/ prefix.
//! Returns 404 for requests without version prefix.

use axum::{
    body::Body,
    http::StatusCode,
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Layer that adds API version enforcement.
#[derive(Clone)]
pub struct ApiVersionLayer;

impl<S> Layer<S> for ApiVersionLayer {
    type Service = ApiVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiVersionService { inner }
    }
}

#[derive(Clone)]
pub struct ApiVersionService<S> {
    inner: S,
}

impl<S> Service<http::Request<Body>> for ApiVersionService<S>
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
        let path = req.uri().path().to_string();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Allow health check endpoints without version prefix
            if path.starts_with("/healthz") || path.starts_with("/readyz") || path.starts_with("/startupz") {
                return inner.call(req).await;
            }

            // All API endpoints must use /v1/ prefix
            if !path.starts_with("/v1/") {
                let response = http::Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "error": "API version not specified. Use /v1/ prefix.",
                        "code": "VERSION_REQUIRED"
                    }).to_string()))
                    .unwrap();
                return Ok(response);
            }

            inner.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_versioned_request_passes() {
        let service = ApiVersionLayer.layer(
            tower::service_fn(|req: http::Request<Body>| async move {
                Ok::<_, std::convert::Infallible>(
                    http::Response::builder().body(Body::from("ok")).unwrap()
                )
            })
        );

        let req = http::Request::builder()
            .uri("/v1/payments")
            .body(Body::empty())
            .unwrap();

        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_unversioned_request_rejected() {
        let service = ApiVersionLayer.layer(
            tower::service_fn(|req: http::Request<Body>| async move {
                Ok::<_, std::convert::Infallible>(
                    http::Response::builder().body(Body::from("ok")).unwrap()
                )
            })
        );

        let req = http::Request::builder()
            .uri("/payments")
            .body(Body::empty())
            .unwrap();

        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_health_check_passes_without_version() {
        let service = ApiVersionLayer.layer(
            tower::service_fn(|req: http::Request<Body>| async move {
                Ok::<_, std::convert::Infallible>(
                    http::Response::builder().body(Body::from("ok")).unwrap()
                )
            })
        );

        let req = http::Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}

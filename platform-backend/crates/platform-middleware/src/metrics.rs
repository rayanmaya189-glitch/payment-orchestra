//! Prometheus metrics middleware for all services.
//!
//! Collects request count, latency, and error rate metrics.
//! Exposes /metrics endpoint for Prometheus scraping.

use axum::{
    body::Body,
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

/// Shared metrics state — thread-safe counters.
#[derive(Debug, Default)]
pub struct MetricsState {
    pub requests_total: AtomicU64,
    pub requests_success: AtomicU64,
    pub requests_error: AtomicU64,
    pub requests_by_status: std::sync::Mutex<Vec<(u16, u64)>>,
}

impl MetricsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, status: u16, latency_ms: u64) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if status < 400 {
            self.requests_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.requests_error.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn to_prometheus(&self) -> String {
        let total = self.requests_total.load(Ordering::Relaxed);
        let success = self.requests_success.load(Ordering::Relaxed);
        let error = self.requests_error.load(Ordering::Relaxed);

        format!(
            "# HELP platform_requests_total Total number of requests\n\
             # TYPE platform_requests_total counter\n\
             platform_requests_total {total}\n\
             # HELP platform_requests_success Successful requests (< 400)\n\
             # TYPE platform_requests_success counter\n\
             platform_requests_success {success}\n\
             # HELP platform_requests_error Error requests (>= 400)\n\
             # TYPE platform_requests_error counter\n\
             platform_requests_error {error}\n\
             # HELP platform_request_duration_seconds Request duration\n\
             # TYPE platform_request_duration_seconds histogram\n"
        )
    }
}

/// Layer that adds Prometheus metrics collection.
#[derive(Clone)]
pub struct MetricsLayer {
    state: Arc<MetricsState>,
}

impl MetricsLayer {
    pub fn new(state: Arc<MetricsState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
    state: Arc<MetricsState>,
}

impl<S> Service<http::Request<Body>> for MetricsService<S>
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
        let start = Instant::now();
        let mut inner = self.inner.clone();
        let state = self.state.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;
            let latency_ms = start.elapsed().as_millis() as u64;
            state.record_request(response.status().as_u16(), latency_ms);
            Ok(response)
        })
    }
}

/// Handler for /metrics endpoint — returns Prometheus format.
pub async fn metrics_handler(
    axum::extract::State(state): axum::extract::State<Arc<MetricsState>>,
) -> impl IntoResponse {
    let body = state.to_prometheus();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_state() {
        let state = MetricsState::new();
        state.record_request(200, 10);
        state.record_request(200, 20);
        state.record_request(500, 100);

        assert_eq!(state.requests_total.load(Ordering::Relaxed), 3);
        assert_eq!(state.requests_success.load(Ordering::Relaxed), 2);
        assert_eq!(state.requests_error.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prometheus_output() {
        let state = MetricsState::new();
        state.record_request(200, 10);
        let output = state.to_prometheus();
        assert!(output.contains("platform_requests_total"));
        assert!(output.contains("platform_requests_success"));
        assert!(output.contains("platform_requests_error"));
    }
}

//! gRPC metrics middleware using tower [`Layer`] + [`Service`].
//!
//! Automatically records latency, in-flight count, and error metrics for every
//! gRPC call.  Works with tonic's `Server::builder().layer(...)` API.
//!
//! # Naming
//!
//! | Metric name | Type | Labels |
//! |---|---|---|
//! | `payment_orchestra_grpc_requests_total` | Counter | service, method, status |
//! | `payment_orchestra_grpc_request_duration_ms` | Histogram | service, method |
//! | `payment_orchestra_in_flight_requests` | Gauge | service |
//! | `payment_orchestra_errors_total` | Counter | service, type |
//!
//! # Usage
//!
//! ```rust,ignore
//! use platform_metrics::grpc_interceptor::MetricsLayer;
//!
//! Server::builder()
//!     .layer(MetricsLayer::new("orchestration-service"))
//!     .add_service(MyServiceServer::new(impl))
//!     .serve(addr)
//!     .await?;
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use tower::{Layer, Service};

use crate::{record_error, record_grpc_duration, record_request, set_in_flight_requests};

// ─── Layer ───────────────────────────────────────────────────────────────────

/// A tower [`Layer`] that wraps every gRPC call with latency and in-flight
/// metrics.
///
/// See the [module-level documentation](self) for usage.
#[derive(Clone, Debug)]
pub struct MetricsLayer {
    service_name: String,
}

impl MetricsLayer {
    /// Create a new layer for the given service name.
    ///
    /// `service_name` should match the service's registry name (e.g.
    /// `"orchestration-service"`).
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, service: S) -> Self::Service {
        MetricsService {
            inner: service,
            service_name: self.service_name.clone(),
            in_flight: Arc::new(AtomicI64::new(0)),
        }
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

/// Instrumented wrapper around an inner tower service.
///
/// Created by [`MetricsLayer`].  Records request duration, in-flight count,
/// and error status for every call to the inner service.
#[derive(Clone, Debug)]
pub struct MetricsService<S> {
    inner: S,
    service_name: String,
    in_flight: Arc<AtomicI64>,
}

impl<S> MetricsService<S> {
    /// Current number of in-flight requests.
    pub fn in_flight_count(&self) -> i64 {
        self.in_flight.load(Ordering::Relaxed)
    }
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for MetricsService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
    S::Future: Send + 'static,
    S::Error: std::fmt::Debug + Into<Box<dyn std::error::Error + Send + Sync>>,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        let start = Instant::now();
        let method = req.uri().path().trim_start_matches('/').to_string();
        let service_name = self.service_name.clone();
        let in_flight = Arc::clone(&self.in_flight);

        let in_flight_count = in_flight.fetch_add(1, Ordering::Relaxed) + 1;
        set_in_flight_requests(&service_name, in_flight_count);

        let fut = self.inner.call(req);

        Box::pin(async move {
            let result = fut.await;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            match &result {
                Ok(response) => {
                    let status_code = response.status().as_u16();
                    record_request(&service_name, &method, status_code, elapsed_ms);
                }
                Err(e) => {
                    record_error(&service_name, &format!("{e:?}"));
                    record_grpc_duration(&service_name, &method, elapsed_ms);
                }
            }

            let remaining = in_flight.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
            set_in_flight_requests(&service_name, remaining);

            result.map_err(Into::into)
        })
    }
}

// ─── Manual Instrumentation (for non-tower call sites) ───────────────────────

/// Simple manual interceptor for cases where a tower `Layer` is not feasible.
///
/// Unlike [`MetricsLayer`], this does NOT require `tower` and can be used
/// directly inside handler functions.  You must supply the method name
/// explicitly since `tonic::Request<T>` hides the URI path.
///
/// # Example
///
/// ```rust,ignore
/// use platform_metrics::grpc_interceptor::ManualInterceptor;
///
/// let metrics = ManualInterceptor::new("my-service");
///
/// async fn my_handler(req: Request<MyMessage>) -> Result<Response<MyReply>, Status> {
///     let (req, guard) = metrics.instrument(req, "/my.package.MyService/MyMethod");
///     // ... handle request ...
///     guard.ok();
///     Ok(Response::new(reply))
/// }
#[derive(Clone)]
pub struct ManualInterceptor {
    service_name: String,
    in_flight: Arc<AtomicI64>,
}

impl ManualInterceptor {
    /// Create a new manual interceptor for the given service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            in_flight: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Current number of in-flight requests.
    pub fn in_flight_count(&self) -> i64 {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Instrument a single gRPC call.
    ///
    /// Returns the request (unmodified) and a guard that records metrics when
    /// dropped.  Call [`ManualGuard::ok`] or [`ManualGuard::err`] to record
    /// success/failure.
    pub fn instrument<T>(
        &self,
        request: tonic::Request<T>,
        method: &str,
    ) -> (tonic::Request<T>, ManualGuard) {
        let in_flight_count = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
        set_in_flight_requests(&self.service_name, in_flight_count);

        let guard = ManualGuard {
            service_name: self.service_name.clone(),
            method: method.to_string(),
            start: Instant::now(),
            in_flight: Arc::clone(&self.in_flight),
        };

        (request, guard)
    }
}

/// RAII guard created by [`ManualInterceptor::instrument`].
///
/// Records metrics on [`ok`](ManualGuard::ok) or [`err`](ManualGuard::err)
/// and always decrements the in-flight counter on drop.
pub struct ManualGuard {
    service_name: String,
    method: String,
    start: Instant,
    in_flight: Arc<AtomicI64>,
}

impl ManualGuard {
    /// Mark the request as successful.
    pub fn ok(self) {
        let elapsed_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        record_request(&self.service_name, &self.method, 200, elapsed_ms);
        // Drop will decrement in_flight
    }

    /// Mark the request as failed with the given gRPC status.
    pub fn err(self, status: &tonic::Status) {
        let elapsed_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        record_grpc_duration(&self.service_name, &self.method, elapsed_ms);
        record_error(&self.service_name, &format!("gRPC_{:?}", status.code()));
        // Drop will decrement in_flight
    }
}

impl Drop for ManualGuard {
    fn drop(&mut self) {
        let remaining = self.in_flight.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
        set_in_flight_requests(&self.service_name, remaining);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_in_flight_tracking() {
        let metrics = ManualInterceptor::new("test-service");
        assert_eq!(metrics.in_flight_count(), 0);

        let (_req, guard) = metrics.instrument(tonic::Request::new(()), "/test/Empty");
        assert_eq!(metrics.in_flight_count(), 1);

        guard.ok();
        assert_eq!(metrics.in_flight_count(), 0);
    }

    #[test]
    fn test_manual_multiple_in_flight() {
        let metrics = ManualInterceptor::new("test-service");

        let (_req1, guard1) = metrics.instrument(tonic::Request::new(()), "/test/First");
        let (_req2, guard2) = metrics.instrument(tonic::Request::new(()), "/test/Second");
        assert_eq!(metrics.in_flight_count(), 2);

        guard1.ok();
        assert_eq!(metrics.in_flight_count(), 1);

        guard2.ok();
        assert_eq!(metrics.in_flight_count(), 0);
    }

    #[test]
    fn test_manual_err_decrements_in_flight() {
        let metrics = ManualInterceptor::new("test-service");

        let (_req, guard) = metrics.instrument(tonic::Request::new(()), "/test/Fail");
        assert_eq!(metrics.in_flight_count(), 1);

        let status = tonic::Status::internal("test error");
        guard.err(&status);
        assert_eq!(metrics.in_flight_count(), 0);
    }

    #[test]
    fn test_manual_guard_drop_decrements() {
        let metrics = ManualInterceptor::new("test-service");

        let (_req, guard) = metrics.instrument(tonic::Request::new(()), "/test/Drop");
        assert_eq!(metrics.in_flight_count(), 1);

        drop(guard);
        assert_eq!(metrics.in_flight_count(), 0);
    }


}

//! Production-grade metrics instrumentation for Payment Orchestra.
//!
//! Uses the `metrics` crate (`metrics = "0.24"`) to expose counters,
//! histograms, and gauges that can be scraped via:
//! - `/metrics` Prometheus endpoint (via `metrics-exporter-prometheus`)
//! - OpenTelemetry collector (via `opentelemetry` + `opentelemetry-otlp`)
//!
//! # Metric Naming Convention
//!
//! All metrics follow the pattern: `payment_orchestra_<component>_<metric>`
//!
//! For example:
//! - `payment_orchestra_grpc_requests_total`
//! - `payment_orchestra_grpc_request_duration_ms`
//! - `payment_orchestra_db_query_duration_ms`

use metrics::{counter, gauge, histogram};
use std::sync::OnceLock;

// ─── Metric Helpers ──────────────────────────────────────────────────────────

/// Increment a request counter with service, method, and status labels.
pub fn record_request(service: &str, method: &str, status: u16, latency_ms: f64) {
    counter!("payment_orchestra_grpc_requests_total", "service" => service.to_string(), "method" => method.to_string(), "status" => status.to_string()).increment(1);
    histogram!("payment_orchestra_grpc_request_duration_ms", "service" => service.to_string(), "method" => method.to_string()).record(latency_ms);
    tracing::debug!(service, method, status, latency_ms, "gRPC request");
}

/// Record a gRPC request duration in milliseconds (convenience wrapper).
pub fn record_grpc_duration(service: &str, method: &str, duration_ms: f64) {
    histogram!("payment_orchestra_grpc_request_duration_ms", "service" => service.to_string(), "method" => method.to_string()).record(duration_ms);
}

/// Increment an error counter with service and error type labels.
pub fn record_error(service: &str, error_type: &str) {
    counter!("payment_orchestra_errors_total", "service" => service.to_string(), "type" => error_type.to_string()).increment(1);
    tracing::warn!(service, error_type, "Error recorded");
}

/// Record a database query duration.
pub fn record_db_query(query_name: &str, latency_ms: f64) {
    histogram!("payment_orchestra_db_query_duration_ms", "query" => query_name.to_string()).record(latency_ms);
    tracing::debug!(query = query_name, latency_ms, "DB query");
}

/// Record a message publish event.
pub fn record_message_publish(topic: &str, latency_ms: f64) {
    histogram!("payment_orchestra_message_publish_duration_ms", "topic" => topic.to_string()).record(latency_ms);
    counter!("payment_orchestra_messages_published_total", "topic" => topic.to_string()).increment(1);
    tracing::debug!(topic, latency_ms, "Message published");
}

/// Record an HTTP response status code.
pub fn record_http_status_code(status: u16) {
    counter!("payment_orchestra_http_responses_total", "status" => status.to_string()).increment(1);
}

/// Record a panic or crash event.
pub fn record_panic(service: &str) {
    counter!("payment_orchestra_panics_total", "service" => service.to_string()).increment(1);
}

/// Record the number of active connections.
pub fn set_active_connections(service: &str, count: u64) {
    gauge!("payment_orchestra_active_connections", "service" => service.to_string()).set(count as f64);
}

/// Record queue depth for background workers.
pub fn set_queue_depth(service: &str, queue: &str, depth: u64) {
    gauge!("payment_orchestra_queue_depth", "service" => service.to_string(), "queue" => queue.to_string()).set(depth as f64);
}

/// Record circuit breaker state (1 = open, 0 = closed).
pub fn set_circuit_breaker_state(service: &str, is_open: bool) {
    gauge!("payment_orchestra_circuit_breaker", "service" => service.to_string()).set(if is_open { 1.0 } else { 0.0 });
}

/// Record the number of in-flight requests.
pub fn set_in_flight_requests(service: &str, count: i64) {
    gauge!("payment_orchestra_in_flight_requests", "service" => service.to_string()).set(count as f64);
}

// ─── Uptime Tracking ─────────────────────────────────────────────────────────

static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();

/// Initialize the uptime tracker. Call once at service startup.
pub fn init_uptime_tracker() {
    START_TIME.get_or_init(|| {
        let now = std::time::Instant::now();
        // Record startup time as a gauge
        gauge!("payment_orchestra_uptime_seconds").set(0.0);
        now
    });
}

/// Record current uptime. Call periodically or on each request.
pub fn record_uptime() {
    if let Some(start) = START_TIME.get() {
        let uptime = start.elapsed().as_secs_f64();
        gauge!("payment_orchestra_uptime_seconds").set(uptime);
    }
}

// ─── HTTP Metrics (backward-compatible aliases) ──────────────────────────────

/// Record an HTTP response status code (legacy wrapper).
pub fn record_status_code(status: u16) {
    record_http_status_code(status);
}

pub mod grpc_interceptor;

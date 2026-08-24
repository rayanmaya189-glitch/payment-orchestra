/// Metrics collection middleware.
pub fn record_request_metrics(_service: &str, _method: &str, _latency_ms: f64, _status: u16) {
    // Delegates to platform-metrics
}

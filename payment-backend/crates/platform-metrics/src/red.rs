/// Simple request counter.
pub fn record_request(service: &str, _method: &str, _status: u16, _latency_ms: f64) {
    tracing::debug!("Request recorded: {}", service);
}

/// Simple error counter.
pub fn record_error(service: &str, error_type: &str) {
    tracing::warn!("Error recorded: {} - {}", service, error_type);
}

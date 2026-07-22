/// HTTP-specific metrics helpers.
pub fn record_status_code(status: u16) {
    tracing::debug!("HTTP response status: {}", status);
}

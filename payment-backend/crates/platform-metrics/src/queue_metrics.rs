/// Message queue latency tracking.
pub fn record_message_publish(topic: &str, latency_ms: f64) {
    tracing::debug!("Published to {} in {}ms", topic, latency_ms);
}

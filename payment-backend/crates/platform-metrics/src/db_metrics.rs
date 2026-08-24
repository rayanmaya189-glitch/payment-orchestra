/// Database query latency tracking.
pub fn record_db_query(query_name: &str, latency_ms: f64) {
    tracing::debug!("DB query {} took {}ms", query_name, latency_ms);
}

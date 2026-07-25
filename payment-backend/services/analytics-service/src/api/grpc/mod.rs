//! gRPC service implementation for analytics-service (BC-15).
//! Translates between protobuf types and domain types for analytics queries.

use chrono::{DateTime, Utc};
use tonic::Status;

use crate::domain::*;


pub mod financial;

pub struct AnalyticsGrpcService<C, Q, R> {
    _commands: C,
    queries: Q,
    repo: R,
}

impl<C, Q, R> AnalyticsGrpcService<C, Q, R> {
    pub fn new(commands: C, queries: Q, repo: R) -> Self {
        Self { _commands: commands, queries, repo }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub(crate) fn resolve_currency(events: &[AnalyticsEvent]) -> String {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for event in events {
        if let Some(ref currency) = event.currency {
            *counts.entry(currency.as_str()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(currency, _)| currency.to_string())
        .unwrap_or_else(|| "AED".to_string())
}

pub(crate) fn parse_timestamp(unix_ms: i64, field: &str) -> Result<DateTime<Utc>, Status> {
    chrono::DateTime::from_timestamp_millis(unix_ms)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid {} timestamp", field)))
}

pub(crate) fn analytics_error_to_status(e: AnalyticsError) -> Status {
    match e {
        AnalyticsError::Unavailable(msg) => Status::unavailable(msg),
        AnalyticsError::StaleData(secs) => {
            Status::unavailable(format!("Stale data: last ingestion was {} seconds ago", secs))
        }
        AnalyticsError::InvalidDateRange(msg) => Status::invalid_argument(msg),
        AnalyticsError::QueryTimeout(msg) => Status::deadline_exceeded(msg),
        AnalyticsError::EventNotFound(id) => {
            Status::not_found(format!("Event not found: {}", id))
        }
        AnalyticsError::InvalidEventType(et) => {
            Status::invalid_argument(format!("Invalid event type: {}", et))
        }
        AnalyticsError::DatabaseError(msg) => {
            Status::internal(format!("Database error: {}", msg))
        }
    }
}

impl From<AnalyticsError> for Status {
    fn from(e: AnalyticsError) -> Self {
        analytics_error_to_status(e)
    }
}

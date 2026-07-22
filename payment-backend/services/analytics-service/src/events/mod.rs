//! Analytics Service events
//!
//! Events that the analytics service itself emits (e.g., when data staleness
//! thresholds are breached or when automated reports are generated).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsServiceEvent {
    EventIngested(IngestionRecord),
    DataStaleDetected(StaleDataAlert),
    ReportGenerated(ReportMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionRecord {
    pub event_id: Uuid,
    pub event_type: String,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleDataAlert {
    pub last_ingestion_at: DateTime<Utc>,
    pub staleness_seconds: i64,
    pub threshold_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub report_id: Uuid,
    pub report_type: String,
    pub generated_at: DateTime<Utc>,
    pub record_count: u64,
}

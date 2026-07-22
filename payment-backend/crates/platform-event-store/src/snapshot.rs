use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot {
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_sequence: i64,
    pub state: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

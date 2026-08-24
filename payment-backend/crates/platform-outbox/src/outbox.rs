use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub outbox_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqEvent {
    pub event_id: Uuid,
    pub stream_name: String,
    pub original_subject: String,
    pub payload: Vec<u8>,
    pub error_message: String,
    pub retry_count: i32,
    pub first_failed_at: DateTime<Utc>,
    pub last_attempt_at: DateTime<Utc>,
    pub status: DlqStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DlqStatus {
    Pending,
    Resolved,
    Discarded,
}

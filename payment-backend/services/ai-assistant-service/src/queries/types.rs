//! AI Assistant Service query types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub operator_id: Uuid,
    pub title: String,
    pub message_count: usize,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiHealthStatus {
    pub available: bool,
    pub message: String,
    pub uptime_hours: u64,
}

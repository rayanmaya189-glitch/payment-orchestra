use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub key: String,
    pub payload_hash: Vec<u8>,
    pub result_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub enum IdempotencyResult {
    /// New idempotency key — caller should process
    New,
    /// Duplicate key with same payload — return cached result
    Duplicate(serde_json::Value),
    /// Same key, different payload — conflict error
    Conflict,
}

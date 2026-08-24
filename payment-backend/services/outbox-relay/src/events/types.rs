//! Outbox Relay events

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutboxRelayEvent {
    EntryPublished(EntryPublishedPayload),
    EntryFailed(EntryFailedPayload),
    RelayCycleCompleted(CycleCompletedPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPublishedPayload {
    pub outbox_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryFailedPayload {
    pub outbox_id: Uuid,
    pub error: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleCompletedPayload {
    pub published: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
}

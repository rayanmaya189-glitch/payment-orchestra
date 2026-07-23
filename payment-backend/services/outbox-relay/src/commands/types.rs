//! Command types for Outbox Relay

use uuid::Uuid;

pub struct AppendEntry {
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Vec<u8>,
}

pub struct PollAndPublish;

pub struct MarkPublished {
    pub outbox_id: Uuid,
}

pub struct StartRelay;

pub struct StopRelay;

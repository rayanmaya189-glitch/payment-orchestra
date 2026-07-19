use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: u32,
    pub occurred_at: DateTime<Utc>,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub payload: serde_json::Value,
    pub trace_context: Option<String>,
}

impl EventEnvelope {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: Uuid,
        event_type: impl Into<String>,
        actor_type: impl Into<String>,
        correlation_id: Uuid,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            aggregate_type: aggregate_type.into(),
            aggregate_id,
            event_type: event_type.into(),
            event_version: 1,
            occurred_at: Utc::now(),
            actor_type: actor_type.into(),
            actor_id: None,
            causation_id: None,
            correlation_id,
            payload,
            trace_context: None,
        }
    }
}

pub trait DomainEvent {
    fn event_type(&self) -> &'static str;
    fn aggregate_type(&self) -> &'static str;
}

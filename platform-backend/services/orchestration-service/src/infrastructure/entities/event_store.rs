use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Append-only event store entity per SRS Part 5, §4 (Event Sourcing).
/// Events are immutable once written — no UPDATE/DELETE allowed.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "event_store")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_id: Uuid,
    /// Aggregate type (e.g., "PaymentIntent", "RoutingPolicy")
    pub aggregate_type: String,
    /// Aggregate root ID
    pub aggregate_id: Uuid,
    /// Monotonically increasing sequence per aggregate (optimistic concurrency)
    pub event_sequence: i64,
    /// Event type (e.g., "PaymentIntentCreated", "PaymentAuthorized")
    pub event_type: String,
    /// Schema version of the event payload
    pub event_version: i32,
    /// JSON-encoded event payload
    pub payload: Json,
    /// ISO 8601 timestamp with millisecond precision
    pub occurred_at: DateTimeWithTimeZone,
    /// Actor that caused this event (user, system, scheduler)
    pub actor_type: String,
    /// Optional actor ID (user UUID, API key ID)
    pub actor_id: Option<Uuid>,
    /// Causation ID — the event that caused this event
    pub causation_id: Option<Uuid>,
    /// Correlation ID — ties together events in a single request
    pub correlation_id: Uuid,
    /// W3C Trace Context for distributed tracing
    pub trace_context: Option<String>,
    /// HMAC-SHA256 signature for event integrity (optional)
    pub signature: Option<Vec<u8>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Convert to shared EventEnvelope for publishing.
    pub fn to_event_envelope(&self) -> shared_types::events::EventEnvelope {
        shared_types::events::EventEnvelope {
            event_id: self.event_id,
            aggregate_type: self.aggregate_type.clone(),
            aggregate_id: self.aggregate_id,
            event_type: self.event_type.clone(),
            event_version: self.event_version as u32,
            occurred_at: self.occurred_at.into(),
            actor_type: self.actor_type.clone(),
            actor_id: self.actor_id,
            causation_id: self.causation_id,
            correlation_id: self.correlation_id,
            payload: self.payload.clone().into(),
            trace_context: self.trace_context.clone(),
            signature: self.signature.clone(),
        }
    }
}

impl From<shared_types::events::EventEnvelope> for ActiveModel {
    fn from(env: shared_types::events::EventEnvelope) -> Self {
        Self {
            event_id: sea_orm::Set(env.event_id),
            aggregate_type: sea_orm::Set(env.aggregate_type),
            aggregate_id: sea_orm::Set(env.aggregate_id),
            event_sequence: sea_orm::NotSet, // Set by repository with optimistic concurrency
            event_type: sea_orm::Set(env.event_type),
            event_version: sea_orm::Set(env.event_version as i32),
            payload: sea_orm::Set(Json::from(env.payload)),
            occurred_at: sea_orm::Set(env.occurred_at.into()),
            actor_type: sea_orm::Set(env.actor_type),
            actor_id: sea_orm::Set(env.actor_id),
            causation_id: sea_orm::Set(env.causation_id),
            correlation_id: sea_orm::Set(env.correlation_id),
            trace_context: sea_orm::Set(env.trace_context),
            signature: sea_orm::Set(env.signature),
        }
    }
}

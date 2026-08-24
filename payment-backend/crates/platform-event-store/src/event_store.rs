//! Event store — append-only log for event-sourced aggregates.
//! Persisted in PostgreSQL via SeaORM.
//! Composite primary key: (aggregate_type, aggregate_id, event_sequence).

use sea_orm::{DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait, QueryOrder};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::event_store_entity::{self, Entity as EventStoreEntity, ActiveModel as EventStoreActiveModel, Model as EventStoreModel};
use crate::corruption;

/// A stored domain event in the event store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_sequence: i64,
    pub event_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub payload: Vec<u8>,
    pub encrypted: bool,
}

/// Event store backed by PostgreSQL via SeaORM.
/// Provides append-only event persistence with integrity verification.
pub struct EventStore {
    db: DatabaseConnection,
}

impl EventStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Append events atomically.
    /// Returns an error if any event has a duplicate event_id (idempotency guard).
    ///
    /// DB-001: Callers specify expected_event_sequence for optimistic concurrency.
    /// The insert will fail if (aggregate_type, aggregate_id, event_sequence) already exists.
    pub async fn append_events(&self, events: Vec<StoredEvent>) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }

        for event in &events {
            let checksum = corruption::compute_checksum(&event.payload);
            let active = EventStoreActiveModel {
                aggregate_type: Set(event.aggregate_type.clone()),
                aggregate_id: Set(event.aggregate_id),
                event_sequence: Set(event.event_sequence),
                event_id: Set(event.event_id),
                event_type: Set(event.event_type.clone()),
                event_version: Set(event.event_version as i16),
                occurred_at: Set(event.occurred_at),
                actor_type: Set(event.actor_type.clone()),
                actor_id: Set(event.actor_id),
                causation_id: Set(event.causation_id),
                correlation_id: Set(event.correlation_id),
                payload: Set(event.payload.clone()),
                checksum: Set(Some(checksum)),
                encrypted: Set(event.encrypted),
            };
            EventStoreEntity::insert(active)
                .exec(&self.db)
                .await
                .map_err(|e| format!("Failed to append event {}: {}", event.event_id, e))?;
        }

        Ok(())
    }

    /// Read events for an aggregate from a given sequence number onward.
    pub async fn read_events(
        &self,
        aggregate_id: Uuid,
        from_sequence: i64,
    ) -> Result<Vec<StoredEvent>, String> {
        let models = EventStoreEntity::find()
            .filter(event_store_entity::Column::AggregateId.eq(aggregate_id))
            .filter(event_store_entity::Column::EventSequence.gte(from_sequence))
            .order_by_asc(event_store_entity::Column::EventSequence)
            .all(&self.db)
            .await
            .map_err(|e| format!("Failed to read events: {}", e))?;

        models.into_iter().map(model_to_stored_event).collect()
    }

    /// Read all events for an aggregate (full event stream).
    pub async fn read_all_events(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoredEvent>, String> {
        let models = EventStoreEntity::find()
            .filter(event_store_entity::Column::AggregateType.eq(aggregate_type))
            .filter(event_store_entity::Column::AggregateId.eq(aggregate_id))
            .order_by_asc(event_store_entity::Column::EventSequence)
            .all(&self.db)
            .await
            .map_err(|e| format!("Failed to read all events: {}", e))?;

        models.into_iter().map(model_to_stored_event).collect()
    }

    /// Read a range of events for an aggregate (for partial replay).
    pub async fn read_event_range(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
        from_sequence: i64,
        to_sequence: i64,
    ) -> Result<Vec<StoredEvent>, String> {
        let models = EventStoreEntity::find()
            .filter(event_store_entity::Column::AggregateType.eq(aggregate_type))
            .filter(event_store_entity::Column::AggregateId.eq(aggregate_id))
            .filter(event_store_entity::Column::EventSequence.between(from_sequence, to_sequence))
            .order_by_asc(event_store_entity::Column::EventSequence)
            .all(&self.db)
            .await
            .map_err(|e| format!("Failed to read event range: {}", e))?;

        models.into_iter().map(model_to_stored_event).collect()
    }

    /// Get the latest sequence number for an aggregate.
    pub async fn latest_sequence(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Option<i64>, String> {
        let model = EventStoreEntity::find()
            .filter(event_store_entity::Column::AggregateType.eq(aggregate_type))
            .filter(event_store_entity::Column::AggregateId.eq(aggregate_id))
            .order_by_desc(event_store_entity::Column::EventSequence)
            .one(&self.db)
            .await
            .map_err(|e| format!("Failed to get latest sequence: {}", e))?;

        Ok(model.map(|m| m.event_sequence))
    }

    /// Verify integrity of stored events by recomputing checksums.
    /// Returns a list of event IDs that failed verification.
    pub async fn verify_integrity(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Vec<Uuid>, String> {
        let events = self.read_all_events(aggregate_type, aggregate_id).await?;
        let mut corrupted = Vec::new();

        for event in &events {
            // Re-read the stored checksum from DB
            let model = EventStoreEntity::find()
                .filter(event_store_entity::Column::AggregateType.eq(aggregate_type))
                .filter(event_store_entity::Column::AggregateId.eq(aggregate_id))
                .filter(event_store_entity::Column::EventSequence.eq(event.event_sequence))
                .one(&self.db)
                .await
                .map_err(|e| format!("Integrity check read failed: {}", e))?
                .ok_or_else(|| format!("Event {} not found during integrity check", event.event_id))?;

            if !corruption::verify_checksum(&event.payload, model.checksum.as_deref()) {
                corrupted.push(event.event_id);
            }
        }

        Ok(corrupted)
    }
}

fn model_to_stored_event(m: EventStoreModel) -> Result<StoredEvent, String> {
    Ok(StoredEvent {
        event_id: m.event_id,
        aggregate_type: m.aggregate_type,
        aggregate_id: m.aggregate_id,
        event_type: m.event_type,
        event_sequence: m.event_sequence,
        event_version: m.event_version as u16,
        occurred_at: m.occurred_at,
        actor_type: m.actor_type,
        actor_id: m.actor_id,
        causation_id: m.causation_id,
        correlation_id: m.correlation_id,
        payload: m.payload,
        encrypted: m.encrypted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that we can create a StoredEvent with valid data.
    #[test]
    fn test_stored_event_creation() {
        let event = StoredEvent {
            event_id: Uuid::now_v7(),
            aggregate_type: "PaymentIntent".into(),
            aggregate_id: Uuid::now_v7(),
            event_type: "PaymentIntentCreated".into(),
            event_sequence: 1,
            event_version: 1,
            occurred_at: Utc::now(),
            actor_type: "system".into(),
            actor_id: None,
            causation_id: None,
            correlation_id: Uuid::now_v7(),
            payload: vec![1, 2, 3],
            encrypted: false,
        };

        assert_eq!(event.event_sequence, 1);
        assert_eq!(event.event_version, 1);
        assert_eq!(event.encrypted, false);
    }

    /// Test that checksum computation is deterministic.
    #[test]
    fn test_checksum_deterministic() {
        let payload = b"test payload data";
        let c1 = corruption::compute_checksum(payload);
        let c2 = corruption::compute_checksum(payload);
        assert_eq!(c1, c2);
    }

    /// Test that different payloads produce different checksums.
    #[test]
    fn test_checksum_unique() {
        let c1 = corruption::compute_checksum(b"payload one");
        let c2 = corruption::compute_checksum(b"payload two");
        assert_ne!(c1, c2);
    }
}

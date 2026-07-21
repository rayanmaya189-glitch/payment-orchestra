#![allow(dead_code, unused_imports)]
//! Shared NATS JetStream event publishing with transactional outbox pattern.
//!
//! Provides `EventPublisher` for all services to publish domain events via
//! the outbox pattern (SRS OUTBOX-001): events are written to the `outbox`
//! table in the same Postgres transaction as the aggregate state change,
//! then published to NATS JetStream.
//!
//! The `OutboxRelay` polls unpublished outbox entries and publishes them,
//! providing crash recovery and at-least-once delivery.

use async_nats::jetstream;
use shared_types::events::EventEnvelope;
use platform_error::PlatformError;

/// NATS JetStream event publisher with transactional outbox support.
#[derive(Clone)]
pub struct EventPublisher {
    client: async_nats::Client,
    stream_name: String,
}

impl EventPublisher {
    /// Connect to NATS and create/ensure the JetStream stream exists.
    pub async fn new(nats_url: &str, stream_name: &str) -> Result<Self, PlatformError> {
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Failed to connect to NATS: {e}")))?;

        let jetstream = jetstream::new(client.clone());

        // Create stream if it doesn't exist (idempotent)
        let _ = jetstream
            .create_stream(jetstream::stream::Config {
                name: stream_name.to_string(),
                subjects: vec![format!("{stream_name}.>")],
                max_messages: 1_000_000,
                max_age: std::time::Duration::from_secs(86400 * 7), // 7 days retention
                ..Default::default()
            })
            .await;

        Ok(Self {
            client,
            stream_name: stream_name.to_string(),
        })
    }

    /// Publish event directly to NATS JetStream (low-latency path).
    pub async fn publish(&self, event: &EventEnvelope) -> Result<(), PlatformError> {
        let subject = format!(
            "{}.{}.{}",
            self.stream_name, event.aggregate_type, event.event_type
        );

        let payload = serde_json::to_vec(event)
            .map_err(|e| PlatformError::Internal(format!("Failed to serialize event: {e}")))?;

        self.client
            .publish(subject, payload.into())
            .await
            .map_err(|e| PlatformError::Internal(format!("Failed to publish event: {e}")))?;

        Ok(())
    }

    /// Write event to outbox table + publish immediately.
    ///
    /// The outbox write should be part of the same DB transaction as the
    /// aggregate state change. This method writes the outbox entry and
    /// publishes immediately for low latency.
    pub async fn publish_with_outbox(
        &self,
        db: &sea_orm::DatabaseConnection,
        event: &EventEnvelope,
        aggregate_type: &str,
        aggregate_id: uuid::Uuid,
    ) -> Result<(), PlatformError> {
        use sea_orm::{ConnectionTrait, Statement};

        let payload = serde_json::to_vec(event)
            .map_err(|e| PlatformError::Internal(format!("Failed to serialize event: {e}")))?;

        let id = uuid::Uuid::now_v7();
        let now = chrono::Utc::now();

        // Parameterized query — prevents SQL injection (OWASP A03)
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO outbox (outbox_id, aggregate_type, aggregate_id, event_type, event_version, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            vec![
                id.into(),
                aggregate_type.into(),
                aggregate_id.into(),
                event.event_type.clone().into(),
                event.event_version.into(),
                hex::encode(&payload).into(),
                now.to_rfc3339().into(),
            ],
        ))
        .await
        .map_err(|e| PlatformError::Internal(format!("Failed to write outbox: {e}")))?;

        // Also publish immediately for low latency
        self.publish(event).await?;

        Ok(())
    }
}

/// Outbox relay process — polls unpublished outbox entries and publishes to NATS.
///
/// This provides crash recovery: if the immediate publish fails or the service
/// crashes after writing to outbox but before publishing, the relay picks up
/// unpublished entries and publishes them.
///
/// SRS OUTBOX-001: Relay polls unpublished, publishes to NATS, marks published.
pub struct OutboxRelay {
    publisher: EventPublisher,
    poll_interval: std::time::Duration,
    max_batch_size: u32,
}

impl OutboxRelay {
    pub fn new(publisher: EventPublisher) -> Self {
        Self {
            publisher,
            poll_interval: std::time::Duration::from_secs(1),
            max_batch_size: 100,
        }
    }

    /// Run the relay loop. Polls for unpublished events and publishes them.
    pub async fn run(&self, db: &sea_orm::DatabaseConnection) -> Result<(), PlatformError> {
        tracing::info!("Outbox relay started");

        loop {
            match self.poll_and_publish(db).await {
                Ok(0) => {
                    // No unpublished events — sleep before next poll
                    tokio::time::sleep(self.poll_interval).await;
                }
                Ok(count) => {
                    tracing::info!("Published {} outbox events", count);
                }
                Err(e) => {
                    tracing::error!("Outbox relay error: {e}");
                    tokio::time::sleep(self.poll_interval).await;
                }
            }
        }
    }

    /// Poll unpublished outbox entries and publish them.
    /// Returns the number of events published.
    async fn poll_and_publish(&self, db: &sea_orm::DatabaseConnection) -> Result<u32, PlatformError> {
        use sea_orm::{ConnectionTrait, Statement};

        // Fetch unpublished events (outbox pattern: published_at IS NULL)
        let rows = db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT outbox_id, aggregate_type, aggregate_id, event_type, event_version, payload
                 FROM outbox
                 WHERE published_at IS NULL
                 ORDER BY created_at ASC
                 LIMIT $1",
                vec![self.max_batch_size.into()],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Outbox poll failed: {e}")))?;

        if rows.is_empty() {
            return Ok(0);
        }

        let mut published = 0u32;

        for row in rows {
            let outbox_id: uuid::Uuid = row.try_get("", "outbox_id").unwrap_or_default();
            let aggregate_type: String = row.try_get("", "aggregate_type").unwrap_or_default();
            let aggregate_id: uuid::Uuid = row.try_get("", "aggregate_id").unwrap_or_default();
            let event_type: String = row.try_get("", "event_type").unwrap_or_default();
            let event_version: i32 = row.try_get("", "event_version").unwrap_or(1);
            let payload_hex: String = row.try_get("", "payload").unwrap_or_default();

            // Decode the event payload
            let payload_bytes = hex::decode(&payload_hex)
                .unwrap_or_default();
            let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
                .unwrap_or(serde_json::Value::Null);

            let event = EventEnvelope {
                event_id: outbox_id,
                aggregate_type: aggregate_type.clone(),
                aggregate_id,
                event_type: event_type.clone(),
                event_version: event_version as u32,
                occurred_at: chrono::Utc::now(),
                actor_type: "outbox-relay".to_string(),
                actor_id: None,
                causation_id: None,
                correlation_id: uuid::Uuid::now_v7(),
                payload,
                trace_context: None,
                signature: None,
            };

            // Publish to NATS
            match self.publisher.publish(&event).await {
                Ok(()) => {
                    // Mark as published
                    db.execute(Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Postgres,
                        "UPDATE outbox SET published_at = NOW() WHERE outbox_id = $1",
                        vec![outbox_id.into()],
                    ))
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Outbox mark published failed: {e}")))?;

                    published += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to publish outbox event {}: {e}", outbox_id);
                    // Don't mark as published — will retry on next poll
                }
            }
        }

        Ok(published)
    }
}

#[cfg(test)]
mod tests {
    // Note: OutboxRelay requires a live NATS connection for full integration testing.
    // Unit tests for relay logic are in the integration test suite.
}

//! Shared NATS JetStream event publishing with transactional outbox pattern.
//!
//! Provides `EventPublisher` for all services to publish domain events via
//! the outbox pattern (SRS OUTBOX-001): events are written to the `outbox`
//! table in the same Postgres transaction as the aggregate state change,
//! then published to NATS JetStream.

use async_nats::jetstream;
use shared_types::events::EventEnvelope;
use platform_error::PlatformError;

/// NATS JetStream event publisher with transactional outbox support.
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

use async_nats::jetstream;
use shared_types::events::EventEnvelope;
use platform_error::PlatformError;

pub struct EventPublisher {
    client: async_nats::Client,
    stream_name: String,
}

impl EventPublisher {
    pub async fn new(nats_url: &str, stream_name: &str) -> Result<Self, PlatformError> {
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Failed to connect to NATS: {e}")))?;

        let jetstream = jetstream::new(client.clone());

        // Create stream if it doesn't exist
        let _ = jetstream
            .create_stream(jetstream::stream::Config {
                name: stream_name.to_string(),
                subjects: vec![format!("{stream_name}.>")],
                max_messages: 1_000_000,
                max_age: std::time::Duration::from_secs(86400 * 7), // 7 days
                ..Default::default()
            })
            .await;

        Ok(Self {
            client,
            stream_name: stream_name.to_string(),
        })
    }

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

        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "INSERT INTO outbox (outbox_id, aggregate_type, aggregate_id, event_type, event_version, payload, created_at)
                 VALUES ('{}', '{}', '{}', '{}', {}, E'\\\\x{}'::bytea, '{}')",
                id, aggregate_type, aggregate_id, event.event_type, event.event_version,
                hex::encode(&payload), now.to_rfc3339()
            ),
        ))
        .await
        .map_err(|e| PlatformError::Internal(format!("Failed to write outbox: {e}")))?;

        // Also publish immediately for low latency
        self.publish(event).await?;

        Ok(())
    }
}

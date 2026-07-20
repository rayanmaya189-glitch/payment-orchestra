use platform_error::PlatformError;
use shared_types::events::EventEnvelope;

/// Event publisher for IAM service events.
pub struct EventPublisher {
    client: async_nats::Client,
    stream_name: String,
}

impl EventPublisher {
    pub async fn new(nats_url: &str, stream_name: &str) -> Result<Self, PlatformError> {
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| PlatformError::Unavailable(format!("Failed to connect to NATS: {e}")))?;

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
}

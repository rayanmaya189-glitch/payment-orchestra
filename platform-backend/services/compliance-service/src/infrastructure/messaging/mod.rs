use platform_error::PlatformError;
use shared_types::events::EventEnvelope;
pub struct EventPublisher { client: async_nats::Client, stream_name: String }
impl EventPublisher {
    pub async fn new(nats_url: &str, stream_name: &str) -> Result<Self, PlatformError> {
        let client = async_nats::connect(nats_url).await.map_err(|e| PlatformError::Unavailable(format!("NATS: {e}")))?;
        Ok(Self { client, stream_name: stream_name.to_string() })
    }
    pub async fn publish(&self, event: &EventEnvelope) -> Result<(), PlatformError> {
        let subject = format!("{}.{}.{}", self.stream_name, event.aggregate_type, event.event_type);
        let payload = serde_json::to_vec(event).map_err(|e| PlatformError::Internal(format!("Serialize: {e}")))?;
        self.client.publish(subject, payload.into()).await.map_err(|e| PlatformError::Internal(format!("Publish: {e}")))?;
        Ok(())
    }
}

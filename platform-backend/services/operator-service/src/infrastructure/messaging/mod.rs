use shared_types::events::EventEnvelope;
use uuid::Uuid;

pub struct EventPublisher {
    // TODO: NATS JetStream connection
}

impl EventPublisher {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn publish(&self, event: &EventEnvelope) -> Result<(), platform_error::PlatformError> {
        // TODO: Publish to NATS JetStream
        let _ = event;
        Ok(())
    }
}

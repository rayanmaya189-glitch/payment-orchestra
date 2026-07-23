use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

// ── EventBus Trait ──────────────────────────────────────────────────────────

/// Generic event bus trait for publishing domain events.
/// Services can use this trait via `Box<dyn EventBus>` for dependency injection.
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    /// Publish a serialized event payload to the given subject.
    async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), String>;
}

// ── Noop Event Bus (for testing) ────────────────────────────────────────────

/// No-op event bus that discards all published events.
/// Use in tests or when the event bus is not yet configured.
#[derive(Clone, Default)]
pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _subject: &str, _payload: Vec<u8>) -> Result<(), String> {
        Ok(())
    }
}

// ── Concrete Channel Event Bus ──────────────────────────────────────────────

/// In-process event bus backed by a `tokio::sync::broadcast` channel.
/// Events are serialized as JSON and broadcast to all subscribers.
pub struct ChannelEventBus {
    sender: broadcast::Sender<Vec<u8>>,
}

impl ChannelEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.sender.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Synchronous publish — wraps the event in an envelope and sends it.
    /// Used by services that call publish in non-async contexts.
    pub fn publish_sync(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        let envelope = EventEnvelope {
            subject: subject.to_string(),
            payload,
            event_id: Uuid::now_v7(),
            timestamp: chrono::Utc::now(),
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
        self.sender.send(bytes).map_err(|_| "No subscribers".to_string())?;
        info!("Published to {}", subject);
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventBus for ChannelEventBus {
    async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        self.publish_sync(subject, payload)
    }
}

// ── Event Envelope ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope {
    pub subject: String,
    pub payload: Vec<u8>,
    pub event_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ── Backward Compatibility Alias ────────────────────────────────────────────

/// Backward-compatible alias. Use `ChannelEventBus` for new code.
pub type EventBusStruct = ChannelEventBus;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_event_bus() {
        let bus = NoopEventBus;
        assert!(bus.publish("test.subject", vec![1, 2, 3]).await.is_ok());
    }

    #[tokio::test]
    async fn test_channel_event_bus_publish_subscribe() {
        let bus = ChannelEventBus::new(16);
        let mut rx = bus.subscribe();

        let payload = vec![1, 2, 3];
        bus.publish("test.subject", payload.clone()).await.unwrap();

        let received = rx.try_recv().unwrap();
        let envelope: EventEnvelope = serde_json::from_slice(&received).unwrap();
        assert_eq!(envelope.subject, "test.subject");
    }

    #[test]
    fn test_channel_event_bus_publish_sync() {
        let bus = ChannelEventBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish_sync("sync.subject", vec![4, 5, 6]).unwrap();

        let received = rx.try_recv().unwrap();
        let envelope: EventEnvelope = serde_json::from_slice(&received).unwrap();
        assert_eq!(envelope.subject, "sync.subject");
    }
}

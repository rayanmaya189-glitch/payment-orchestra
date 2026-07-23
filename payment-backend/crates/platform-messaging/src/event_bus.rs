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
#[derive(Clone, Default)]
pub struct NoopEventBus;

#[async_trait::async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _subject: &str, _payload: Vec<u8>) -> Result<(), String> {
        Ok(())
    }
}

// ── Event Envelope (protobuf-encoded in transport) ──────────────────────────

/// Transport envelope wrapping event subject and payload.
/// Encoded with `prost::Message::encode` for protobuf-native transport.
#[derive(Clone, PartialEq, prost::Message)]
pub struct EventEnvelope {
    /// Event subject/topic (e.g., "operator.operator_registered")
    #[prost(string, tag = "1")]
    pub subject: String,
    /// Serialized event payload (protobuf-encoded domain event)
    #[prost(bytes, tag = "2")]
    pub payload: Vec<u8>,
    /// Unique event identifier (UUIDv7 as string)
    #[prost(string, tag = "3")]
    pub event_id: String,
    /// Event timestamp in milliseconds since Unix epoch
    #[prost(int64, tag = "4")]
    pub timestamp_unix_ms: i64,
}

impl EventEnvelope {
    /// Create a new event envelope with the given subject and payload.
    pub fn new(subject: &str, payload: Vec<u8>) -> Self {
        Self {
            subject: subject.to_string(),
            payload,
            event_id: Uuid::now_v7().to_string(),
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Decode an event envelope from protobuf-encoded bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        prost::Message::decode(bytes).map_err(|e| e.to_string())
    }

    /// Encode this envelope into protobuf bytes for transport.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        prost::Message::encode(self, &mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }
}

// ── Concrete Channel Event Bus ──────────────────────────────────────────────

/// In-process event bus backed by a `tokio::sync::broadcast` channel.
/// Events are encoded as protobuf and broadcast to all subscribers.
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

    /// Synchronous publish — wraps subject + payload in a protobuf-encoded
    /// EventEnvelope and broadcasts to all subscribers.
    pub fn publish_sync(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        let envelope = EventEnvelope::new(subject, payload);
        let bytes = envelope.to_bytes()?;
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

// ── Fire-and-Forget Publish Helper ──────────────────────────────────────────

use std::sync::Arc;

/// Publish an event payload via the event bus in a fire-and-forget manner.
///
/// If an event bus is configured, spawns a background task that publishes the
/// encoded payload to the constructed subject `"{prefix}.{event_type}"`.
/// On failure, a warning is logged. This helper is designed for command handlers
/// that need to publish events without awaiting the publish result.
///
/// # Arguments
/// * `event_bus` - Optional reference to an `Arc<dyn EventBus>`.
/// * `subject_prefix` - The domain prefix (e.g., `"operator"`, `"compliance"`).
/// * `event_type` - The specific event type name (e.g., `"operator_registered"`).
/// * `payload` - The protobuf-encoded event payload bytes.
pub fn publish_event_fire_and_forget(
    event_bus: &Option<Arc<dyn EventBus>>,
    subject_prefix: &str,
    event_type: &str,
    payload: Vec<u8>,
) {
    if let Some(bus) = event_bus.as_ref() {
        let subject = format!("{}.{}", subject_prefix, event_type);
        let bus = Arc::clone(bus);
        tokio::spawn(async move {
            if let Err(e) = bus.publish(&subject, payload).await {
                tracing::warn!(subject = %subject, error = %e, "Failed to publish event");
            }
        });
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

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
        bus.publish_sync("test.subject", payload).unwrap();

        let received = rx.try_recv().unwrap();
        let envelope = EventEnvelope::from_bytes(&received).unwrap();
        assert_eq!(envelope.subject, "test.subject");
        assert!(!envelope.event_id.is_empty());
    }

    #[test]
    fn test_event_envelope_roundtrip() {
        let envelope = EventEnvelope::new("roundtrip.test", vec![4, 5, 6]);
        let bytes = envelope.to_bytes().unwrap();
        let decoded = EventEnvelope::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.subject, envelope.subject);
        assert_eq!(decoded.payload, envelope.payload);
        assert_eq!(decoded.event_id, envelope.event_id);
    }

    // ── Fire-and-Forget Helper Tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_fire_and_forget_with_noop_bus() {
        // Verify that publishing to a NoopEventBus via the helper does not panic.
        let bus: Arc<dyn EventBus> = Arc::new(NoopEventBus);
        publish_event_fire_and_forget(&Some(bus), "test", "event_type", vec![1, 2, 3]);
        // Give the spawned background task time to complete.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_fire_and_forget_with_none_bus() {
        // Verify that passing None does not panic and takes no action.
        publish_event_fire_and_forget(&None, "test", "event_type", vec![1, 2, 3]);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_fire_and_forget_payload_integrity() {
        // Verify that the helper publishes the correct subject and payload
        // by using a ChannelEventBus and subscribing to verify.
        let channel = ChannelEventBus::new(16);
        let mut rx = channel.subscribe();
        let bus: Arc<dyn EventBus> = Arc::new(channel);

        let payload = vec![10, 20, 30];
        publish_event_fire_and_forget(&Some(bus), "test-prefix", "my.event", payload.clone());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let received = rx.try_recv().unwrap();
        let envelope = EventEnvelope::from_bytes(&received).unwrap();
        assert_eq!(envelope.subject, "test-prefix.my.event");
        assert_eq!(envelope.payload, payload);
    }
}

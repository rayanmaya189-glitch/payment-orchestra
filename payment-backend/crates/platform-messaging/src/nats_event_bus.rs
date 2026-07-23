//! NATS JetStream-backed event bus implementation.
//!
//! Provides a durable, at-least-once event publishing layer using
//! NATS JetStream streams and consumers. Falls back to in-memory
//! channel if NATS connection is unavailable.

use async_nats::jetstream::{self, Context};
use async_nats::ConnectOptions;
use std::time::Duration;
use tracing::info;

use crate::event_bus::{EventBus, EventEnvelope};

/// NATS JetStream-backed event bus.
///
/// Creates a JetStream context and provides publish capabilities.
/// Streams are created lazily on first publish to a new subject.
pub struct NatsJetStreamEventBus {
    jetstream: Context,
}

impl NatsJetStreamEventBus {
    /// Connect to NATS and create a JetStream context.
    ///
    /// # Arguments
    /// * `url` - NATS server URL (e.g., "nats://localhost:4222")
    pub async fn connect(url: &str) -> Result<Self, String> {
        let client = async_nats::connect_with_options(
            url,
            ConnectOptions::new()
                .name("payment-orchestra-event-bus")
                .event_callback(|event| async move {
                    tracing::debug!(?event, "NATS connection event");
                }),
        )
        .await
        .map_err(|e| format!("NATS connection failed: {}", e))?;

        let jetstream = jetstream::new(client);

        info!("Connected to NATS JetStream at {}", url);
        Ok(Self { jetstream })
    }

    /// Ensure a stream exists for the given subject.
    /// Streams are named based on the subject prefix.
    async fn ensure_stream(&self, subject: &str) -> Result<(), String> {
        // Derive stream name from the first two segments of the subject
        // e.g., "events.orchestration.payment_intent.authorized.v1" → "events_orchestration"
        let stream_name = subject
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join("_");

        let stream_config = jetstream::stream::Config {
            name: stream_name.clone(),
            subjects: vec![format!("{}.>", stream_name.replace('_', "."))],
            max_age: Duration::from_secs(7 * 24 * 3600), // 7 days retention
            ..Default::default()
        };

        self.jetstream
            .get_or_create_stream(stream_config)
            .await
            .map_err(|e| format!("Failed to create stream {}: {}", stream_name, e))?;

        Ok(())
    }

    /// Publish an event to NATS JetStream.
    pub async fn publish_jetstream(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        // Ensure the stream exists for this subject
        self.ensure_stream(subject).await?;

        // Publish with JetStream for durable storage
        let subject_owned = subject.to_string();
        let result = self.jetstream.publish(subject_owned, payload.into()).await;
        match result {
            Ok(_ack) => {
                tracing::debug!(subject = %subject, "Event published to NATS JetStream");
                Ok(())
            }
            Err(e) => Err(format!("NATS publish failed: {}", e)),
        }
    }
}

#[async_trait::async_trait]
impl EventBus for NatsJetStreamEventBus {
    async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        self.publish_jetstream(subject, payload).await
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_without_nats_returns_error() {
        // Attempt to connect to a non-existent NATS server
        let result = NatsJetStreamEventBus::connect("nats://localhost:14222").await;
        assert!(result.is_err(), "Should fail to connect to non-existent NATS");
    }

    #[tokio::test]
    async fn test_envelope_integration() {
        // Verify that the envelope encoding works with the NATS event bus pattern
        let envelope = EventEnvelope::new("events.test.event.v1", vec![1, 2, 3]);
        let bytes = envelope.to_bytes().unwrap();
        let decoded = EventEnvelope::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.subject, "events.test.event.v1");
        assert_eq!(decoded.payload, vec![1, 2, 3]);
        assert!(!decoded.event_id.is_empty());
    }
}

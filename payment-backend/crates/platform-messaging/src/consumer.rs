//! NATS JetStream consumer — durable event consumption with acknowledgment.
//!
//! Provides:
//! - Durable pull consumers for at-least-once delivery
//! - Automatic acknowledgment on success
//! - Dead-letter queue integration for failed events
//! - Configurable max delivery attempts

use async_nats::jetstream::{self, consumer::PullConsumer, AckKind};
use async_nats::jetstream::consumer::pull::Config as ConsumerConfig;
use futures::StreamExt;
use std::time::Duration;
use tracing::{info, warn, error};

use crate::dlq::{DlqEvent, DlqStatus};
use crate::event_bus::EventBus;

/// A durable NATS JetStream consumer for a specific stream.
pub struct EventConsumer {
    /// Consumer name (for dedup/diagnostics).
    pub name: String,
    /// Stream name to consume from.
    pub stream_name: String,
    /// Optional filter subject (e.g., "events.orchestration.>").
    pub filter_subject: Option<String>,
    /// Maximum delivery attempts before sending to DLQ.
    pub max_deliver: i64,
}

impl EventConsumer {
    pub fn new(name: &str, stream_name: &str) -> Self {
        Self {
            name: name.to_string(),
            stream_name: stream_name.to_string(),
            filter_subject: None,
            max_deliver: 3,
        }
    }

    pub fn with_filter(mut self, subject: &str) -> Self {
        self.filter_subject = Some(subject.to_string());
        self
    }

    /// Create a pull consumer on the given JetStream context.
    pub async fn create_consumer(
        &self,
        jetstream: &jetstream::Context,
    ) -> Result<PullConsumer, String> {
        let mut config = ConsumerConfig {
            durable_name: Some(self.name.clone()),
            max_deliver: self.max_deliver,
            ack_wait: Duration::from_secs(30),
            max_ack_pending: 1000,
            ..Default::default()
        };

        if let Some(ref subject) = self.filter_subject {
            config.filter_subject = subject.clone();
        }

        let stream = jetstream
            .get_stream(&self.stream_name)
            .await
            .map_err(|e| format!("Failed to get stream {}: {}", self.stream_name, e))?;

        let consumer = stream
            .create_consumer(config)
            .await
            .map_err(|e| format!("Failed to create consumer {}: {}", self.name, e))?;

        info!(
            consumer = %self.name,
            stream = %self.stream_name,
            filter = ?self.filter_subject,
            max_deliver = self.max_deliver,
            "NATS JetStream consumer created"
        );

        Ok(consumer)
    }

    /// Process messages using `fetch().messages()` — fetches a batch of messages
    /// and processes each one.
    pub async fn process_batch_loop<F, Fut>(
        &self,
        consumer: PullConsumer,
        dlq_bus: Option<&dyn EventBus>,
        handler: F,
    ) where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send,
    {
        info!(consumer = %self.name, "Starting consumer batch processing loop");

        loop {
            match consumer.fetch().max_messages(10).expires(Duration::from_secs(5)).messages().await {
                Ok(mut batch) => {
                    while let Some(Ok(msg)) = batch.next().await {
                        let payload = msg.payload.to_vec();
                        let subject = msg.subject.to_string();

                        match handler(payload).await {
                            Ok(()) => {
                                if let Err(e) = msg.ack().await {
                                    warn!(error = %e, "Failed to ack message");
                                }
                            }
                            Err(e) => {
                                warn!(
                                    consumer = %self.name,
                                    subject = %subject,
                                    error = %e,
                                    "Message processing failed"
                                );

                                // Get delivery count from message info
                                let delivered = msg.info().map(|i| i.delivered).unwrap_or(0);

                                if delivered >= self.max_deliver {
                                    // Publish to DLQ subject
                                    let dlq_subject = format!("events.dlq.{}", self.stream_name);
                                    let dlq_payload = DlqEvent {
                                        event_id: uuid::Uuid::now_v7(),
                                        stream_name: self.stream_name.clone(),
                                        original_subject: subject,
                                        payload: msg.payload.to_vec(),
                                        error_message: e,
                                        retry_count: delivered as i32,
                                        first_failed_at: chrono::Utc::now(),
                                        last_attempt_at: chrono::Utc::now(),
                                        status: DlqStatus::Pending,
                                    };
                                    let dlq_bytes = serde_json::to_vec(&dlq_payload).unwrap_or_default();

                                    if let Some(bus) = dlq_bus {
                                        if let Err(dlq_err) = bus.publish(&dlq_subject, dlq_bytes).await {
                                            error!(error = %dlq_err, "Failed to publish to DLQ");
                                        }
                                    }

                                    // Send term ack to prevent further redeliveries
                                    if let Err(e) = msg.ack_with(AckKind::Term).await {
                                        warn!(error = %e, "Failed to term message");
                                    }
                                } else {
                                    // Nak with no delay to trigger retry
                                    if let Err(e) = msg.ack_with(AckKind::Nak(None)).await {
                                        warn!(error = %e, "Failed to nak message");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if !err_str.contains("timeout") && !err_str.contains("TIMEOUT") {
                        error!(error = %e, consumer = %self.name, "Consumer batch error");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumer_creation() {
        let consumer = EventConsumer::new("test-consumer", "events_orchestration");
        assert_eq!(consumer.name, "test-consumer");
        assert_eq!(consumer.stream_name, "events_orchestration");
        assert_eq!(consumer.max_deliver, 3);
    }

    #[test]
    fn test_consumer_with_filter() {
        let consumer = EventConsumer::new("test", "stream")
            .with_filter("events.orchestration.>");
        assert_eq!(consumer.filter_subject, Some("events.orchestration.>".into()));
    }
}

//! Outbox Relay commands

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

pub struct AppendEntry {
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Vec<u8>,
}

pub struct PollAndPublish;

pub struct MarkPublished {
    pub outbox_id: Uuid,
}

pub struct StartRelay;

pub struct StopRelay;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn append_entry(&self, cmd: AppendEntry) -> Result<OutboxEntry, OutboxRelayError>;
    async fn poll_and_publish(&self, cmd: PollAndPublish) -> Result<PublishResult, OutboxRelayError>;
    async fn mark_published(&self, cmd: MarkPublished) -> Result<(), OutboxRelayError>;
    async fn start_relay(&self) -> Result<(), OutboxRelayError>;
    async fn stop_relay(&self) -> Result<(), OutboxRelayError>;
}

pub struct OutboxCommandHandler<R: OutboxRepository> {
    repo: R,
    config: OutboxRelayConfig,
    running: Arc<RwLock<bool>>,
}

impl<R: OutboxRepository> OutboxCommandHandler<R> {
    pub fn new(repo: R, config: OutboxRelayConfig) -> Self {
        Self { repo, config, running: Arc::new(RwLock::new(false)) }
    }
}

#[async_trait]
impl<R: OutboxRepository + Send + Sync> CommandHandler for OutboxCommandHandler<R> {
    async fn append_entry(&self, cmd: AppendEntry) -> Result<OutboxEntry, OutboxRelayError> {
        let entry = OutboxEntry {
            outbox_id: Uuid::now_v7(),
            aggregate_type: cmd.aggregate_type,
            aggregate_id: cmd.aggregate_id,
            event_type: cmd.event_type,
            event_version: cmd.event_version,
            payload: cmd.payload,
            created_at: Utc::now(),
            published_at: None,
        };
        self.repo.save_entry(&entry).await?;
        Ok(entry)
    }

    async fn poll_and_publish(&self, _cmd: PollAndPublish) -> Result<PublishResult, OutboxRelayError> {
        let start = std::time::Instant::now();
        let unpublished = self.repo.find_unpublished(self.config.batch_size).await?;

        let mut published = 0u32;
        let mut failed = 0u32;

        for entry in &unpublished {
            // Simulate publishing to NATS
            tracing::debug!("Publishing event: {} for aggregate {}", entry.event_type, entry.aggregate_id);
            if let Err(_e) = self.repo.mark_published(entry.outbox_id).await {
                failed += 1;
            } else {
                published += 1;
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(PublishResult {
            published_count: published,
            failed_count: failed,
            skipped_count: 0,
            duration_ms: elapsed,
        })
    }

    async fn mark_published(&self, cmd: MarkPublished) -> Result<(), OutboxRelayError> {
        self.repo.mark_published(cmd.outbox_id).await
    }

    async fn start_relay(&self) -> Result<(), OutboxRelayError> {
        let mut running = self.running.write().await;
        if *running {
            return Err(OutboxRelayError::AlreadyRunning);
        }
        *running = true;
        Ok(())
    }

    async fn stop_relay(&self) -> Result<(), OutboxRelayError> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(OutboxRelayError::NotRunning);
        }
        *running = false;
        Ok(())
    }
}

//! Outbox Relay public API

pub mod grpc;

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct OutboxRelayApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl OutboxRelayApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self { command_handler: ch, query_handler: qh }
    }

    pub async fn append_entry(&self, cmd: AppendEntry) -> Result<OutboxEntry, OutboxRelayError> {
        self.command_handler.append_entry(cmd).await
    }
    pub async fn poll_and_publish(&self) -> Result<PublishResult, OutboxRelayError> {
        self.command_handler.poll_and_publish(PollAndPublish).await
    }
    pub async fn mark_published(&self, cmd: MarkPublished) -> Result<(), OutboxRelayError> {
        self.command_handler.mark_published(cmd).await
    }
    pub async fn start_relay(&self) -> Result<(), OutboxRelayError> {
        self.command_handler.start_relay().await
    }
    pub async fn stop_relay(&self) -> Result<(), OutboxRelayError> {
        self.command_handler.stop_relay().await
    }
    pub async fn get_metrics(&self) -> Result<RelayMetrics, OutboxRelayError> {
        self.query_handler.get_metrics().await
    }
    pub async fn get_entry(&self, id: Uuid) -> Result<OutboxEntry, OutboxRelayError> {
        self.query_handler.get_entry(id).await
    }
    pub async fn list_unpublished(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        self.query_handler.list_unpublished().await
    }
}

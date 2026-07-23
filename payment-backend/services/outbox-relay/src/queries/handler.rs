//! Outbox Relay query handlers

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_metrics(&self) -> Result<RelayMetrics, OutboxRelayError>;
    async fn get_entry(&self, outbox_id: Uuid) -> Result<OutboxEntry, OutboxRelayError>;
    async fn list_unpublished(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError>;
    async fn get_config(&self) -> OutboxRelayConfig;
}

pub struct OutboxQueryHandler<R: OutboxRepository> {
    repo: R,
    config: OutboxRelayConfig,
    metrics: Arc<RwLock<RelayMetrics>>,
}

impl<R: OutboxRepository> OutboxQueryHandler<R> {
    pub fn new(repo: R, config: OutboxRelayConfig, metrics: Arc<RwLock<RelayMetrics>>) -> Self {
        Self { repo, config, metrics }
    }
}

#[async_trait]
impl<R: OutboxRepository + Send + Sync> QueryHandler for OutboxQueryHandler<R> {
    async fn get_metrics(&self) -> Result<RelayMetrics, OutboxRelayError> {
        let queue_depth = self.repo.count_unpublished().await?;
        let mut metrics = self.metrics.write().await;
        metrics.queue_depth = queue_depth;
        Ok(metrics.clone())
    }

    async fn get_entry(&self, outbox_id: Uuid) -> Result<OutboxEntry, OutboxRelayError> {
        self.repo.load_entry(outbox_id).await?.ok_or(OutboxRelayError::EntryNotFound(outbox_id))
    }

    async fn list_unpublished(&self) -> Result<Vec<OutboxEntry>, OutboxRelayError> {
        self.repo.find_unpublished(u32::MAX).await
    }

    async fn get_config(&self) -> OutboxRelayConfig {
        self.config.clone()
    }
}

//! Outbox relay — polls unpublished outbox entries and publishes them via the event bus.
//!
//! DB-005/006: The outbox entry is written in the same PostgreSQL transaction as the
//! event_store append. This guarantees that if an event is committed, it will eventually
//! be published to NATS — no events are silently lost.
//!
//! DB-007: Outbox entries are purged after `published_at` + retention period.

use std::sync::Arc;
use std::time::Duration;

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, QuerySelect, PaginatorTrait, Set};
use tracing::{info, warn, error};

use platform_messaging::EventBus;

use crate::outbox_entity::{self, Entity as OutboxEntity, ActiveModel as OutboxActiveModel};

/// Outbox relay configuration.
pub struct OutboxRelayConfig {
    /// How often to poll for unpublished entries.
    pub poll_interval: Duration,
    /// Maximum entries to publish in a single batch.
    pub batch_size: u32,
    /// How long after publishing to retain entries (for dedup/recovery).
    pub retention: Duration,
}

impl Default for OutboxRelayConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            batch_size: 100,
            retention: Duration::from_secs(7 * 24 * 3600), // 7 days
        }
    }
}

/// Outbox relay that polls the database and publishes events via NATS.
pub struct OutboxRelay {
    db: DatabaseConnection,
    event_bus: Arc<dyn EventBus>,
    config: OutboxRelayConfig,
}

impl OutboxRelay {
    pub fn new(db: DatabaseConnection, event_bus: Arc<dyn EventBus>, config: OutboxRelayConfig) -> Self {
        Self { db, event_bus, config }
    }

    /// Start the relay loop. Runs until a shutdown signal is received.
    pub async fn run(&self, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
        info!(
            poll_interval = ?self.config.poll_interval,
            batch_size = self.config.batch_size,
            "Outbox relay started"
        );

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Outbox relay shutting down");
                        break;
                    }
                }
                _ = tokio::time::sleep(self.config.poll_interval) => {
                    if let Err(e) = self.publish_batch().await {
                        error!(error = %e, "Outbox relay batch failed");
                    }
                }
            }
        }
    }

    /// Publish a single batch of unpublished outbox entries.
    async fn publish_batch(&self) -> Result<(), String> {
        // Find unpublished entries, oldest first
        let entries = OutboxEntity::find()
            .filter(outbox_entity::Column::Published.eq(false))
            .order_by_asc(outbox_entity::Column::CreatedAt)
            .limit(self.config.batch_size as u64)
            .all(&self.db)
            .await
            .map_err(|e| format!("Failed to query outbox entries: {}", e))?;

        if entries.is_empty() {
            return Ok(());
        }

        let batch_size = entries.len();
        let mut published_count = 0u32;

        for entry in &entries {
            // Build the full NATS subject: e.g. "events.orchestration.payment_intent.authorized.v1"
            let subject = if entry.subject.is_empty() {
                // Legacy entries without explicit subject — derive from event_type
                entry.event_type.clone()
            } else {
                entry.subject.clone()
            };

            match self.event_bus.publish(&subject, entry.payload.clone()).await {
                Ok(()) => {
                    // Mark as published
                    let mut active: OutboxActiveModel = entry.clone().into();
                    active.published = Set(true);
                    active.published_at = Set(Some(chrono::Utc::now()));
                    OutboxEntity::update(active)
                        .exec(&self.db)
                        .await
                        .map_err(|e| format!("Failed to update outbox entry: {}", e))?;
                    published_count += 1;
                }
                Err(e) => {
                    warn!(
                        entry_id = %entry.entry_id,
                        subject = %subject,
                        error = %e,
                        "Failed to publish outbox entry"
                    );
                }
            }
        }

        if published_count > 0 {
            info!(
                published = published_count,
                batch_size = batch_size,
                "Outbox batch published"
            );
        }

        // Purge old published entries
        self.purge_old_entries().await?;

        Ok(())
    }

    /// Purge entries that were published beyond the retention period.
    async fn purge_old_entries(&self) -> Result<(), String> {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(self.config.retention)
            .map_err(|e| format!("Duration conversion: {}", e))?;

        let result = OutboxEntity::delete_many()
            .filter(outbox_entity::Column::Published.eq(true))
            .filter(outbox_entity::Column::PublishedAt.lte(Some(cutoff)))
            .exec(&self.db)
            .await
            .map_err(|e| format!("Failed to purge old entries: {}", e))?;

        if result.rows_affected > 0 {
            info!(purged = result.rows_affected, "Purged old outbox entries");
        }

        Ok(())
    }

    /// Get the current relay lag (number of unpublished entries).
    pub async fn lag(&self) -> Result<i64, String> {
        let count = OutboxEntity::find()
            .filter(outbox_entity::Column::Published.eq(false))
            .count(&self.db)
            .await
            .map_err(|e| format!("Failed to count unpublished entries: {}", e))?;
        Ok(count as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OutboxRelayConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.retention, Duration::from_secs(7 * 24 * 3600));
    }
}

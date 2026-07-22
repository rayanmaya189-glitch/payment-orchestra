//! Analytics Service repository — in-memory ClickHouse simulation
//!
//! For Phase 1, events are stored in-memory. In production, this would
//! be backed by ClickHouse with materialized views for aggregation queries.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::*;

// ---------------------------------------------------------------------------
// AnalyticsRepository trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    /// Store an incoming analytics event.
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError>;
    /// Retrieve all events within a date range.
    async fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    /// Retrieve events by type within a date range.
    async fn get_events_by_type(
        &self,
        event_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    /// Retrieve events by acquirer within a date range.
    async fn get_events_by_acquirer(
        &self,
        acquirer_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    /// Get the timestamp of the most recently ingested event.
    async fn last_ingested_at(&self) -> Option<DateTime<Utc>>;
    /// Get total event count.
    async fn event_count(&self) -> u64;
}

// ---------------------------------------------------------------------------
// InMemoryAnalyticsStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InMemoryAnalyticsStore {
    events: Arc<RwLock<Vec<AnalyticsEvent>>>,
}

impl InMemoryAnalyticsStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl AnalyticsRepository for InMemoryAnalyticsStore {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError> {
        let mut events = self.events.write().await;
        events.push(event.clone());
        Ok(())
    }

    async fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.occurred_at >= start && e.occurred_at <= end)
            .cloned()
            .collect())
    }

    async fn get_events_by_type(
        &self,
        event_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.event_type == event_type && e.occurred_at >= start && e.occurred_at <= end)
            .cloned()
            .collect())
    }

    async fn get_events_by_acquirer(
        &self,
        acquirer_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| {
                e.acquirer_id.as_deref() == Some(acquirer_id)
                    && e.occurred_at >= start
                    && e.occurred_at <= end
            })
            .cloned()
            .collect())
    }

    async fn last_ingested_at(&self) -> Option<DateTime<Utc>> {
        let events = self.events.read().await;
        events.iter().map(|e| e.ingested_at).max()
    }

    async fn event_count(&self) -> u64 {
        let events = self.events.read().await;
        events.len() as u64
    }
}

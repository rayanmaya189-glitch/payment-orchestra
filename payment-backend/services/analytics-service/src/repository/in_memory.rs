//! In-memory Analytics repository (ClickHouse simulation for Phase 1).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::*;
use crate::repository::traits::AnalyticsRepository;

#[derive(Clone)]
pub struct InMemoryAnalyticsStore {
    pub(super) events: Arc<RwLock<Vec<AnalyticsEvent>>>,
}

impl Default for InMemoryAnalyticsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAnalyticsStore {
    pub fn new() -> Self {
        Self { events: Arc::new(RwLock::new(Vec::new())) }
    }
}

#[async_trait]
impl AnalyticsRepository for InMemoryAnalyticsStore {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError> {
        let mut events = self.events.write().await;
        events.push(event.clone());
        Ok(())
    }

    async fn get_events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events.iter().filter(|e| e.occurred_at >= start && e.occurred_at <= end).cloned().collect())
    }

    async fn get_events_by_type(&self, event_type: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events.iter().filter(|e| e.event_type == event_type && e.occurred_at >= start && e.occurred_at <= end).cloned().collect())
    }

    async fn get_events_by_acquirer(&self, acquirer_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError> {
        let events = self.events.read().await;
        Ok(events.iter().filter(|e| e.acquirer_id.as_deref() == Some(acquirer_id) && e.occurred_at >= start && e.occurred_at <= end).cloned().collect())
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

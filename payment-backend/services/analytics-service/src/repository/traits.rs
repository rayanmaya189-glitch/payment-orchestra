//! Analytics Service repository trait.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::*;

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    async fn store_event(&self, event: &AnalyticsEvent) -> Result<(), AnalyticsError>;
    async fn get_events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    async fn get_events_by_type(&self, event_type: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    async fn get_events_by_acquirer(&self, acquirer_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AnalyticsEvent>, AnalyticsError>;
    async fn last_ingested_at(&self) -> Option<DateTime<Utc>>;
    async fn event_count(&self) -> u64;
}

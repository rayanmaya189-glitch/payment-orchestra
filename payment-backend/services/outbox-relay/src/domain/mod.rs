//! Outbox Relay domain model
//!
//! Polls the outbox table and publishes events to the message bus.
//! Implements ADR-011 (Transactional Outbox).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export from platform-outbox
pub use platform_outbox::outbox::OutboxEntry;

// ---------------------------------------------------------------------------
// OutboxRelayConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxRelayConfig {
    pub poll_interval_ms: u64,
    pub batch_size: u32,
    pub max_retries: u32,
}

impl Default for OutboxRelayConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
            batch_size: 100,
            max_retries: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// RelayMetrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMetrics {
    pub total_polled: u64,
    pub total_published: u64,
    pub total_failed: u64,
    pub total_duplicates_skipped: u64,
    pub last_polled_at: Option<DateTime<Utc>>,
    pub queue_depth: u64,
    pub is_running: bool,
}

// ---------------------------------------------------------------------------
// PublishResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub published_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum OutboxRelayError {
    #[error("Outbox entry not found: {0}")]
    EntryNotFound(Uuid),
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    #[error("Maximum retries ({0}) exceeded for entry")]
    MaxRetriesExceeded(u32),
    #[error("Relay is not running")]
    NotRunning,
    #[error("Relay is already running")]
    AlreadyRunning,
}

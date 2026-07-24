//! Dunning (retry) attempt types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single dunning (retry) attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DunningRetry {
    pub retry_number: i32,
    pub scheduled_at: DateTime<Utc>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub status: DunningStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DunningStatus {
    Pending,
    Attempted,
    Successful,
    Exhausted,
}

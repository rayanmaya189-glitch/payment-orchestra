//! FeeVariance aggregate (AGG-04).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::FeeVarianceStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVariance {
    pub variance_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub variance_minor: i64,
    pub variance_percent: f64,
    pub is_within_tolerance: bool,
    pub tolerance_threshold_percent: f64,
    pub status: FeeVarianceStatus,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_note: Option<String>,
}

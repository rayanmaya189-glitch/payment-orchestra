//! SettlementExpectation aggregate (AGG-03).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::ExpectationStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementExpectation {
    pub expectation_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
    pub status: ExpectationStatus,
    pub settled_amount_minor: Option<i64>,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

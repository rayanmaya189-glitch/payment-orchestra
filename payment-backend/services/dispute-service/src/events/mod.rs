//! Dispute Management domain events — BC-10

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisputeEvent {
    Received(ChargebackReceived),
    RepresentmentSubmitted(RepresentmentSubmitted),
    Resolved(ChargebackResolved),
}

/// Event type string constants.
pub const EVENT_TYPE_RECEIVED: &str = "chargeback.received";
pub const EVENT_TYPE_REPRESENTMENT_SUBMITTED: &str = "chargeback.representment_submitted";
pub const EVENT_TYPE_RESOLVED: &str = "chargeback.resolved";

impl DisputeEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Received(_) => EVENT_TYPE_RECEIVED,
            Self::RepresentmentSubmitted(_) => EVENT_TYPE_REPRESENTMENT_SUBMITTED,
            Self::Resolved(_) => EVENT_TYPE_RESOLVED,
        }
    }
}

// ---------------------------------------------------------------------------
// Event payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackReceived {
    pub chargeback_id: Uuid,
    pub operator_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub representment_deadline: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentmentSubmitted {
    pub chargeback_id: Uuid,
    pub submission_id: Uuid,
    pub evidence_description: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackResolved {
    pub chargeback_id: Uuid,
    pub outcome: String,
    pub resolution_note: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

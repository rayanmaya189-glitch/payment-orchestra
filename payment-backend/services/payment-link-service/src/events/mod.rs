//! Payment Link domain events — BC-07

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// All events that a PaymentLink aggregate can produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentLinkEvent {
    /// A new payment link was created.
    Created(PaymentLinkCreated),
    /// The payment link was resolved via successful checkout.
    Resolved(PaymentLinkResolved),
    /// The payment link expired (time-based or manual).
    Expired(PaymentLinkExpired),
    /// The payment link was manually cancelled.
    Cancelled(PaymentLinkCancelled),
}

/// Event type string constants (for routing / persistence).
pub const EVENT_TYPE_CREATED: &str = "payment_link.created";
pub const EVENT_TYPE_RESOLVED: &str = "payment_link.resolved";
pub const EVENT_TYPE_EXPIRED: &str = "payment_link.expired";
pub const EVENT_TYPE_CANCELLED: &str = "payment_link.cancelled";

impl PaymentLinkEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => EVENT_TYPE_CREATED,
            Self::Resolved(_) => EVENT_TYPE_RESOLVED,
            Self::Expired(_) => EVENT_TYPE_EXPIRED,
            Self::Cancelled(_) => EVENT_TYPE_CANCELLED,
        }
    }
}

// ---------------------------------------------------------------------------
// Event payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinkCreated {
    pub payment_link_id: Uuid,
    pub operator_id: Uuid,
    pub token: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub description: Option<String>,
    pub invoice_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinkResolved {
    pub payment_link_id: Uuid,
    pub payment_intent_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinkExpired {
    pub payment_link_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinkCancelled {
    pub payment_link_id: Uuid,
    pub reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

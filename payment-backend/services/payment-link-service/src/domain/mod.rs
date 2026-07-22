//! Payment Link domain model — BC-07
//!
//! Hosted checkout payment links with cryptographically-secure tokens.
//! Thin layer over orchestration-service (BC-05) for payment initiation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// PaymentLink aggregate
// ---------------------------------------------------------------------------

/// Lifecycle status of a payment link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentLinkStatus {
    /// Link is active and can be used for checkout.
    Active,
    /// Payment has been completed via this link.
    Used,
    /// Link has passed its expiration time.
    Expired,
    /// Link was manually cancelled before use.
    Cancelled,
}

impl PaymentLinkStatus {
    /// Returns true if transitioning from `self` to `target` is allowed.
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use PaymentLinkStatus::*;
        matches!(
            (self, target),
            (Active, Used)
                | (Active, Expired)
                | (Active, Cancelled)
        )
    }
}

impl std::fmt::Display for PaymentLinkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Used => write!(f, "used"),
            Self::Expired => write!(f, "expired"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Core PaymentLink aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLink {
    pub payment_link_id: Uuid,
    pub operator_id: Uuid,
    pub token: String,
    pub status: PaymentLinkStatus,
    pub amount_minor_units: i64,
    pub currency: String,
    pub description: Option<String>,
    pub invoice_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub payment_intent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl PaymentLink {
    /// Create a new `PaymentLink` in `Active` status.
    pub fn new(
        operator_id: Uuid,
        token: String,
        amount_minor_units: i64,
        currency: String,
        description: Option<String>,
        invoice_id: Option<Uuid>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, PaymentLinkError> {
        if amount_minor_units <= 0 {
            return Err(PaymentLinkError::InvalidLinkAmount);
        }
        let now = Utc::now();
        Ok(Self {
            payment_link_id: Uuid::now_v7(),
            operator_id,
            token,
            status: PaymentLinkStatus::Active,
            amount_minor_units,
            currency,
            description,
            invoice_id,
            expires_at,
            used_at: None,
            payment_intent_id: None,
            created_at: now,
        })
    }

    /// Mark this link as used (after successful checkout).
    pub fn mark_used(&mut self, payment_intent_id: Uuid) -> Result<(), PaymentLinkError> {
        if self.status == PaymentLinkStatus::Expired {
            return Err(PaymentLinkError::LinkExpired);
        }
        if self.status == PaymentLinkStatus::Used {
            return Err(PaymentLinkError::LinkAlreadyUsed);
        }
        if self.status == PaymentLinkStatus::Cancelled {
            return Err(PaymentLinkError::LinkCancelled);
        }
        if !self.status.can_transition_to(&PaymentLinkStatus::Used) {
            return Err(PaymentLinkError::InvalidTransition);
        }
        self.status = PaymentLinkStatus::Used;
        self.used_at = Some(Utc::now());
        self.payment_intent_id = Some(payment_intent_id);
        Ok(())
    }

    /// Mark this link as expired.
    pub fn mark_expired(&mut self) -> Result<(), PaymentLinkError> {
        if self.status == PaymentLinkStatus::Expired {
            return Ok(()); // idempotent
        }
        if !self.status.can_transition_to(&PaymentLinkStatus::Expired) {
            return Err(PaymentLinkError::InvalidTransition);
        }
        self.status = PaymentLinkStatus::Expired;
        Ok(())
    }

    /// Check whether this link is expired based on current time.
    pub fn is_expired(&self, now: &DateTime<Utc>) -> bool {
        *now >= self.expires_at
    }
}

// ---------------------------------------------------------------------------
// Money value object (minimal for this context)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Payment link not found")]
    NotFound,
    #[error("Payment link has expired")]
    LinkExpired,
    #[error("Payment link has already been used")]
    LinkAlreadyUsed,
    #[error("Payment link has been cancelled")]
    LinkCancelled,
    #[error("Invalid link amount: must be positive")]
    InvalidLinkAmount,
    #[error("Invalid status transition")]
    InvalidTransition,
}

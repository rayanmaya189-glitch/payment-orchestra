use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::OrchestrationError;
use super::payment_status::PaymentStatus;
use super::value_objects::{DeclineReason, Money, PaymentPurpose, SourceType};

use crate::events::PaymentEvent;

// ─── Routing Attempt ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingAttempt {
    pub attempt_id: Uuid,
    pub payment_intent_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_snapshot: Option<String>,
    pub connector_id: String,
    pub status: AttemptStatus,
    pub decline_reason: Option<DeclineReason>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub fee_calculated: Option<String>,
    pub attempted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttemptStatus {
    Approved,
    Declined,
    Timeout,
    Requires3DS,
    PartialApproval,
}

// ─── PaymentIntent Aggregate ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: PaymentStatus,
    pub requested_amount: Money,
    pub authorized_amount: Money,
    pub captured_amount: Money,
    pub refunded_amount: Money,
    pub currency: String,
    pub idempotency_key: String,
    pub payment_method_token_id: Option<Uuid>,
    pub routing_policy_id: Option<Uuid>,
    pub deployment_epoch: i32,
    pub purpose: PaymentPurpose,
    pub metadata: Option<serde_json::Value>,

    // Source Context
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,

    // Risk Score
    pub risk_score: Option<f64>,
    pub risk_level: Option<String>,

    // Settlement Timing
    pub expected_settlement_date: Option<String>,
    pub settlement_cycle: Option<String>,

    // Gateway Profile Link
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_version: Option<i32>,
    pub gateway_rotation_strategy: Option<String>,
    pub gateway_selection_reason: Option<String>,

    // Routing attempts (history)
    pub routing_attempts: Vec<RoutingAttempt>,

    // Event-sourcing version & tracking
    pub version: i64,
    /// Events that have been applied but not yet persisted to the event store.
    /// Populated by apply_event(), drained by the event-sourced repository on save.
    #[serde(default)]
    pub pending_events: Vec<PaymentEvent>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentIntent {
    pub fn new(
        payment_intent_id: Uuid,
        operator_id: Uuid,
        amount: Money,
        purpose: PaymentPurpose,
        idempotency_key: String,
        source_type: SourceType,
        source_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            payment_intent_id,
            operator_id,
            status: PaymentStatus::Created,
            requested_amount: amount.clone(),
            authorized_amount: Money::zero(&amount.currency),
            captured_amount: Money::zero(&amount.currency),
            refunded_amount: Money::zero(&amount.currency),
            currency: amount.currency.clone(),
            idempotency_key,
            payment_method_token_id: None,
            routing_policy_id: None,
            deployment_epoch: 0,
            purpose,
            metadata,
            source_type: Some(source_type.to_string()),
            source_id,
            risk_score: None,
            risk_level: None,
            expected_settlement_date: None,
            settlement_cycle: None,
            gateway_profile_id: None,
            gateway_profile_version: None,
            gateway_rotation_strategy: None,
            gateway_selection_reason: None,
            routing_attempts: Vec::new(),
            version: 0,
            pending_events: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Apply a state transition event to evolve the aggregate.
    pub fn apply_event(&mut self, event: &PaymentEvent) {
        self.pending_events.push(event.clone());
        match event {
            PaymentEvent::PaymentIntentCreated(e) => {
                self.status = PaymentStatus::Created;
                self.requested_amount = Money { amount_minor_units: e.amount_minor_units, currency: e.currency.clone() };
                self.authorized_amount = Money::zero(&e.currency);
                self.captured_amount = Money::zero(&e.currency);
                self.refunded_amount = Money::zero(&e.currency);
                self.currency = e.currency.clone();
                self.source_type = e.source_type.clone();
                self.source_id = e.source_id;
                self.purpose = if e.is_card_verification { PaymentPurpose::CardVerification } else { PaymentPurpose::Payment };
                self.created_at = e.occurred_at;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentAuthorizationAttempted(e) => {
                self.status = PaymentStatus::Authorizing;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentAuthorized(e) => {
                self.status = PaymentStatus::Authorized;
                self.authorized_amount = Money { amount_minor_units: e.authorized_amount_minor, currency: self.currency.clone() };
                self.gateway_profile_id = e.gateway_profile_id;
                self.gateway_profile_version = e.gateway_profile_version;
                self.gateway_rotation_strategy = e.rotation_strategy.clone();
                self.gateway_selection_reason = e.selection_reason.clone();
                self.expected_settlement_date = e.expected_settlement_date.clone();
                self.settlement_cycle = e.settlement_cycle.clone();
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentCaptured(e) => {
                self.status = PaymentStatus::Captured;
                let current_captured = self.captured_amount.amount_minor_units;
                self.captured_amount = Money {
                    amount_minor_units: current_captured + e.captured_amount_minor,
                    currency: self.currency.clone(),
                };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentPartiallyCaptured(e) => {
                self.status = PaymentStatus::PartiallyCaptured;
                let current_captured = self.captured_amount.amount_minor_units;
                self.captured_amount = Money {
                    amount_minor_units: current_captured + e.captured_amount_minor,
                    currency: self.currency.clone(),
                };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentFailed(e) => {
                self.status = PaymentStatus::Failed;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentFailedAllRoutes(e) => {
                self.status = PaymentStatus::FailedAllRoutes;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentVoided(e) => {
                self.status = PaymentStatus::Voided;
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentRefunded(e) => {
                self.status = PaymentStatus::Refunded;
                self.refunded_amount = Money { amount_minor_units: e.refund_amount_minor, currency: self.currency.clone() };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            PaymentEvent::PaymentPartiallyRefunded(e) => {
                self.status = PaymentStatus::PartiallyRefunded;
                self.refunded_amount = Money { amount_minor_units: e.refund_amount_minor, currency: self.currency.clone() };
                self.updated_at = e.occurred_at;
                self.version += 1;
            }
            _ => {} // Non-PaymentIntent events
        }
    }

    /// Check INV-01: captured_amount <= authorized_amount
    pub fn check_capture_invariant(&self, capture_amount: &Money) -> Result<(), OrchestrationError> {
        let total_after = self.captured_amount.checked_add(capture_amount)?;
        if total_after.amount_minor_units > self.authorized_amount.amount_minor_units {
            return Err(OrchestrationError::InvariantViolation("INV-01: capture would exceed authorized amount".into()));
        }
        Ok(())
    }

    /// Check remaining refundable balance
    pub fn remaining_refundable(&self) -> Result<Money, OrchestrationError> {
        self.captured_amount.checked_sub(&self.refunded_amount)
    }
}

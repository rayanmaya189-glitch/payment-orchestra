//! Command type definitions for dispute-service.

use uuid::Uuid;

use crate::domain::*;

// ─── Command Input Structs ───────────────────────────────────────────────────

/// Record a new chargeback from an acquirer notification.
pub struct RecordChargebackCommand {
    pub operator_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    /// Whether the payment intent is in captured state (INV-08).
    pub is_captured: bool,
}

/// Submit representment evidence for a chargeback case.
pub struct SubmitRepresentmentCommand {
    pub chargeback_id: Uuid,
    pub evidence: RepresentmentEvidence,
}

/// Resolve a chargeback case with an outcome.
pub struct ResolveChargebackCommand {
    pub chargeback_id: Uuid,
    pub outcome: ChargebackOutcome,
    pub resolution_note: Option<String>,
}

//! Command type definitions for subscription-service.

use uuid::Uuid;

use crate::domain::*;

// ─── Command Input Structs ───────────────────────────────────────────────────

/// Create a new subscription for a customer.
pub struct CreateSubscriptionCommand {
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub plan: SubscriptionPlan,
    pub payment_method_token_id: Option<Uuid>,
    pub max_dunning_retries: Option<i32>,
}

/// Cancel an active subscription.
/// Must check INV-SUB-01: no running renewal saga.
pub struct CancelSubscriptionCommand {
    pub subscription_id: Uuid,
    pub reason: Option<String>,
    /// Whether a renewal saga is currently running.
    pub has_running_renewal: bool,
}

/// Pause an active subscription.
pub struct PauseSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Resume a paused subscription.
pub struct ResumeSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Trigger a renewal (scheduled job).
pub struct RenewSubscriptionCommand {
    pub subscription_id: Uuid,
}

/// Record a successful payment for a renewal.
pub struct ConfirmRenewalPaymentCommand {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub payment_intent_id: Uuid,
}

/// Record a failed renewal payment (triggers dunning).
pub struct FailRenewalPaymentCommand {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub reason: String,
}

// ─── Command Result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RenewResult {
    pub subscription_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub idempotency_key: String,
    pub amount_minor_units: i64,
    pub currency: String,
}

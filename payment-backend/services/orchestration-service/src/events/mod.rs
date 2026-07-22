//! Domain events for the orchestration-service.
//! All events that can be emitted by the PaymentIntent, RoutingPolicy, and PaymentMethodToken aggregates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All domain events emitted by the orchestration service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentEvent {
    // ── PaymentIntent Events ──────────────────────────────────────────────
    PaymentIntentCreated(PaymentIntentCreated),
    PaymentAuthorizationAttempted(PaymentAuthorizationAttempted),
    PaymentAuthorized(PaymentAuthorized),
    PaymentCaptured(PaymentCaptured),
    PaymentPartiallyCaptured(PaymentPartiallyCaptured),
    PaymentFailed(PaymentFailed),
    PaymentFailedAllRoutes(PaymentFailedAllRoutes),
    PaymentVoided(PaymentVoided),
    PaymentRefunded(PaymentRefunded),
    PaymentPartiallyRefunded(PaymentPartiallyRefunded),

    // ── Routing Policy Events ─────────────────────────────────────────────
    RoutingPolicyActivated(RoutingPolicyActivated),
    RoutingPolicyDeactivated(RoutingPolicyDeactivated),

    // ── PaymentMethodToken Events ─────────────────────────────────────────
    PaymentMethodTokenStored(PaymentMethodTokenStored),
    PaymentMethodTokenExpired(PaymentMethodTokenExpired),
    PaymentMethodTokenRevoked(PaymentMethodTokenRevoked),

    // ── Risk Events ───────────────────────────────────────────────────────
    RiskScoreAssigned(RiskScoreAssigned),

    // ── Gateway Linking Events ────────────────────────────────────────────
    GatewayProfileSelected(GatewayProfileSelected),
}

// ─── PaymentIntent Events ────────────────────────────────────────────────────

/// EVT-01: PaymentIntent has been created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentCreated {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub is_card_verification: bool,
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-02: An authorization attempt was made on a specific acquirer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAuthorizationAttempted {
    pub payment_intent_id: Uuid,
    pub attempt_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub connector_id: String,
    pub declined: bool,
    pub decline_reason: Option<String>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-03: Payment has been authorized by an acquirer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAuthorized {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub acquirer_reference: String,
    pub authorized_amount_minor: i64,
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_version: Option<i32>,
    pub rotation_strategy: Option<String>,
    pub selection_reason: Option<String>,
    pub expected_settlement_date: Option<String>,
    pub settlement_cycle: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-04: Payment has been fully captured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCaptured {
    pub payment_intent_id: Uuid,
    pub captured_amount_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-05: Payment has been partially captured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPartiallyCaptured {
    pub payment_intent_id: Uuid,
    pub captured_amount_minor: i64,
    pub remaining_authorized_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-06: A single authorization attempt failed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailed {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub decline_reason: String,
    pub attempt_number: i32,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-07: All authorization routes have been exhausted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailedAllRoutes {
    pub payment_intent_id: Uuid,
    pub attempts: Vec<FailedAttemptInfo>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAttemptInfo {
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub decline_reason: String,
}

/// EVT-08: Payment has been voided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentVoided {
    pub payment_intent_id: Uuid,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-09: Payment has been fully refunded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRefunded {
    pub payment_intent_id: Uuid,
    pub refund_amount_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-10: Payment has been partially refunded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPartiallyRefunded {
    pub payment_intent_id: Uuid,
    pub refund_amount_minor: i64,
    pub remaining_refundable_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

// ─── Routing Policy Events ───────────────────────────────────────────────────

/// EVT-11: A routing policy has been activated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicyActivated {
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub rules_hash: String,
    pub occurred_at: DateTime<Utc>,
}

/// EVT-12: A routing policy has been deactivated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicyDeactivated {
    pub routing_policy_id: Uuid,
    pub version: i32,
    pub occurred_at: DateTime<Utc>,
}

// ─── PaymentMethodToken Events ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodTokenStored {
    pub token_id: Uuid,
    pub operator_id: Uuid,
    pub payment_method_type: String,
    pub last_four: String,
    pub card_brand: Option<String>,
    pub acquirer_link_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodTokenExpired {
    pub token_id: Uuid,
    pub last_four: String,
    pub acquirer_link_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodTokenRevoked {
    pub token_id: Uuid,
    pub last_four: String,
    pub acquirer_link_id: Uuid,
    pub revocation_reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

// ─── Risk Events ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScoreAssigned {
    pub payment_intent_id: Uuid,
    pub risk_score: f64,
    pub risk_level: String,
    pub rule_version: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

// ─── Gateway Profile Selection Event ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProfileSelected {
    pub payment_intent_id: Uuid,
    pub gateway_profile_id: Uuid,
    pub connector_id: String,
    pub rotation_strategy: String,
    pub selection_reason: String,
    pub fee_calculated: GatewayFeeInfo,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayFeeInfo {
    pub fee_minor: i64,
    pub currency: String,
}

// ─── Event type string constants ─────────────────────────────────────────────

pub mod event_types {
    pub const PAYMENT_INTENT_CREATED: &str = "payment_intent.created";
    pub const PAYMENT_AUTHORIZATION_ATTEMPTED: &str = "payment_intent.authorization_attempted";
    pub const PAYMENT_AUTHORIZED: &str = "payment_intent.authorized";
    pub const PAYMENT_CAPTURED: &str = "payment_intent.captured";
    pub const PAYMENT_PARTIALLY_CAPTURED: &str = "payment_intent.partially_captured";
    pub const PAYMENT_FAILED: &str = "payment_intent.failed";
    pub const PAYMENT_FAILED_ALL_ROUTES: &str = "payment_intent.failed_all_routes";
    pub const PAYMENT_VOIDED: &str = "payment_intent.voided";
    pub const PAYMENT_REFUNDED: &str = "payment_intent.refunded";
    pub const PAYMENT_PARTIALLY_REFUNDED: &str = "payment_intent.partially_refunded";
    pub const ROUTING_POLICY_ACTIVATED: &str = "routing_policy.activated";
    pub const ROUTING_POLICY_DEACTIVATED: &str = "routing_policy.deactivated";
    pub const PAYMENT_METHOD_TOKEN_STORED: &str = "payment_method_token.stored";
    pub const PAYMENT_METHOD_TOKEN_EXPIRED: &str = "payment_method_token.expired";
    pub const PAYMENT_METHOD_TOKEN_REVOKED: &str = "payment_method_token.revoked";
    pub const RISK_SCORE_ASSIGNED: &str = "payment_intent.risk_score_assigned";
    pub const GATEWAY_PROFILE_SELECTED: &str = "payment_intent.gateway_profile_selected";
}

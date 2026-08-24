//! Event type definitions for orchestration-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Event type string constants ─────────────────────────────────────────────

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

// ─── Event enum ──────────────────────────────────────────────────────────────

/// All domain events emitted by the orchestration service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentEvent {
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
    RoutingPolicyActivated(RoutingPolicyActivated),
    RoutingPolicyDeactivated(RoutingPolicyDeactivated),
    PaymentMethodTokenStored(PaymentMethodTokenStored),
    PaymentMethodTokenExpired(PaymentMethodTokenExpired),
    PaymentMethodTokenRevoked(PaymentMethodTokenRevoked),
    RiskScoreAssigned(RiskScoreAssigned),
    GatewayProfileSelected(GatewayProfileSelected),
}

impl PaymentEvent {
    /// Returns the event type string constant for this event.
    pub fn event_type(&self) -> &'static str {
        match self {
            PaymentEvent::PaymentIntentCreated(_) => PAYMENT_INTENT_CREATED,
            PaymentEvent::PaymentAuthorizationAttempted(_) => PAYMENT_AUTHORIZATION_ATTEMPTED,
            PaymentEvent::PaymentAuthorized(_) => PAYMENT_AUTHORIZED,
            PaymentEvent::PaymentCaptured(_) => PAYMENT_CAPTURED,
            PaymentEvent::PaymentPartiallyCaptured(_) => PAYMENT_PARTIALLY_CAPTURED,
            PaymentEvent::PaymentFailed(_) => PAYMENT_FAILED,
            PaymentEvent::PaymentFailedAllRoutes(_) => PAYMENT_FAILED_ALL_ROUTES,
            PaymentEvent::PaymentVoided(_) => PAYMENT_VOIDED,
            PaymentEvent::PaymentRefunded(_) => PAYMENT_REFUNDED,
            PaymentEvent::PaymentPartiallyRefunded(_) => PAYMENT_PARTIALLY_REFUNDED,
            PaymentEvent::RoutingPolicyActivated(_) => ROUTING_POLICY_ACTIVATED,
            PaymentEvent::RoutingPolicyDeactivated(_) => ROUTING_POLICY_DEACTIVATED,
            PaymentEvent::PaymentMethodTokenStored(_) => PAYMENT_METHOD_TOKEN_STORED,
            PaymentEvent::PaymentMethodTokenExpired(_) => PAYMENT_METHOD_TOKEN_EXPIRED,
            PaymentEvent::PaymentMethodTokenRevoked(_) => PAYMENT_METHOD_TOKEN_REVOKED,
            PaymentEvent::RiskScoreAssigned(_) => RISK_SCORE_ASSIGNED,
            PaymentEvent::GatewayProfileSelected(_) => GATEWAY_PROFILE_SELECTED,
        }
    }

    /// Returns the occurred_at timestamp for this event.
    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            PaymentEvent::PaymentIntentCreated(e) => e.occurred_at,
            PaymentEvent::PaymentAuthorizationAttempted(e) => e.occurred_at,
            PaymentEvent::PaymentAuthorized(e) => e.occurred_at,
            PaymentEvent::PaymentCaptured(e) => e.occurred_at,
            PaymentEvent::PaymentPartiallyCaptured(e) => e.occurred_at,
            PaymentEvent::PaymentFailed(e) => e.occurred_at,
            PaymentEvent::PaymentFailedAllRoutes(e) => e.occurred_at,
            PaymentEvent::PaymentVoided(e) => e.occurred_at,
            PaymentEvent::PaymentRefunded(e) => e.occurred_at,
            PaymentEvent::PaymentPartiallyRefunded(e) => e.occurred_at,
            PaymentEvent::RoutingPolicyActivated(e) => e.occurred_at,
            PaymentEvent::RoutingPolicyDeactivated(e) => e.occurred_at,
            PaymentEvent::PaymentMethodTokenStored(e) => e.occurred_at,
            PaymentEvent::PaymentMethodTokenExpired(e) => e.occurred_at,
            PaymentEvent::PaymentMethodTokenRevoked(e) => e.occurred_at,
            PaymentEvent::RiskScoreAssigned(e) => e.occurred_at,
            PaymentEvent::GatewayProfileSelected(e) => e.occurred_at,
        }
    }
}

// ─── PaymentIntent Events ────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCaptured {
    pub payment_intent_id: Uuid,
    pub captured_amount_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPartiallyCaptured {
    pub payment_intent_id: Uuid,
    pub captured_amount_minor: i64,
    pub remaining_authorized_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailed {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub decline_reason: String,
    pub attempt_number: i32,
    pub occurred_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentVoided {
    pub payment_intent_id: Uuid,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRefunded {
    pub payment_intent_id: Uuid,
    pub refund_amount_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPartiallyRefunded {
    pub payment_intent_id: Uuid,
    pub refund_amount_minor: i64,
    pub remaining_refundable_minor: i64,
    pub acquirer_reference: String,
    pub occurred_at: DateTime<Utc>,
}

// ─── Routing Policy Events ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicyActivated {
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub rules_hash: String,
    pub occurred_at: DateTime<Utc>,
}

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

// ─── Gateway Profile Selection ───────────────────────────────────────────────

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

//! Command type definitions for orchestration-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::*;

// ─── PaymentIntent Commands ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreatePaymentIntent {
    pub operator_id: Uuid,
    pub idempotency_key: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub purpose: PaymentPurpose,
    pub metadata: Option<serde_json::Value>,
    pub source_type: SourceType,
    pub source_id: Option<Uuid>,
    pub payment_method_token_id: Option<Uuid>,
    pub preferred_gateway_profile_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AuthorizePaymentIntent {
    pub payment_intent_id: Uuid,
    pub payment_method_token_id: Uuid,
    pub card_scheme: String,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CapturePaymentIntent {
    pub payment_intent_id: Uuid,
    pub amount_minor_units: Option<i64>,
    pub supports_partial_capture: bool,
    pub max_partial_captures: u32,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct VoidPaymentIntent {
    pub payment_intent_id: Uuid,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RefundPaymentIntent {
    pub payment_intent_id: Uuid,
    pub amount_minor_units: i64,
    pub actor_id: Uuid,
}

// ─── Routing Policy Commands ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActivateRoutingPolicy {
    pub operator_id: Uuid,
    pub rules: Vec<RoutingRule>,
    pub failover_config: FailoverConfig,
    pub partial_auth_strategy: PartialAuthStrategy,
    pub rotation_strategy: RotationStrategy,
    pub max_transaction_amount_minor: Option<i64>,
}

// ─── PaymentMethodToken Commands ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StorePaymentMethodToken {
    pub operator_id: Uuid,
    pub payment_method_type: String,
    pub last_four: String,
    pub card_brand: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub acquirer_link_id: Uuid,
    pub acquirer_token_reference: String,
    pub encrypted_token: Vec<u8>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ExpirePaymentMethodToken {
    pub token_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RevokePaymentMethodToken {
    pub token_id: Uuid,
    pub reason: Option<String>,
}

// ─── Command Results ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentResult {
    pub payment_intent_id: Uuid,
    pub status: PaymentStatus,
    pub requested_amount: Money,
    pub authorized_amount: Money,
    pub captured_amount: Money,
    pub refunded_amount: Money,
    pub routing_attempts: Vec<RoutingAttempt>,
    pub events: Vec<PaymentEvent>,
}

#[derive(Debug, Clone)]
pub struct RoutingPolicyResult {
    pub routing_policy_id: Uuid,
    pub version: i32,
    pub status: PolicyStatus,
    pub event: PaymentEvent,
}

#[derive(Debug, Clone)]
pub struct TokenResult {
    pub token_id: Uuid,
    pub event: PaymentEvent,
}

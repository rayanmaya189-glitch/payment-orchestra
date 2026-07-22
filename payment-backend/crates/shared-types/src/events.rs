use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::actor::ActorReference;

/// Common envelope for all domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub actor: ActorReference,
    pub causation_id: Uuid,
    pub correlation_id: Uuid,
    pub payload: Vec<u8>,
    pub trace_context: Option<String>,
    pub signature: Option<Vec<u8>>,
}

impl EventEnvelope {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: Uuid,
        event_type: impl Into<String>,
        event_version: u16,
        actor: ActorReference,
        causation_id: Uuid,
        correlation_id: Uuid,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            aggregate_type: aggregate_type.into(),
            aggregate_id,
            event_type: event_type.into(),
            event_version,
            occurred_at: Utc::now(),
            actor,
            causation_id,
            correlation_id,
            payload,
            trace_context: None,
            signature: None,
        }
    }
}

/// Standard event type constants
pub mod event_types {
    pub const PAYMENT_INTENT_CREATED: &str = "PaymentIntentCreated";
    pub const PAYMENT_AUTHORIZATION_ATTEMPTED: &str = "PaymentAuthorizationAttempted";
    pub const PAYMENT_AUTHORIZED: &str = "PaymentAuthorized";
    pub const PAYMENT_CAPTURED: &str = "PaymentCaptured";
    pub const PAYMENT_PARTIALLY_CAPTURED: &str = "PaymentPartiallyCaptured";
    pub const PAYMENT_FAILED: &str = "PaymentFailed";
    pub const PAYMENT_FAILED_ALL_ROUTES: &str = "PaymentFailedAllRoutes";
    pub const PAYMENT_VOIDED: &str = "PaymentVoided";
    pub const PAYMENT_REFUNDED: &str = "PaymentRefunded";
    pub const PAYMENT_PARTIALLY_REFUNDED: &str = "PaymentPartiallyRefunded";
    pub const ROUTING_POLICY_ACTIVATED: &str = "RoutingPolicyActivated";
    pub const ROUTING_POLICY_DEACTIVATED: &str = "RoutingPolicyDeactivated";
    pub const OPERATOR_REGISTERED: &str = "OperatorRegistered";
    pub const OPERATOR_VERIFIED: &str = "OperatorVerified";
    pub const OPERATOR_SUSPENDED: &str = "OperatorSuspended";
    pub const PRINCIPAL_CREATED: &str = "PrincipalCreated";
    pub const PRINCIPAL_AUTHENTICATED: &str = "PrincipalAuthenticated";
    pub const API_KEY_CREATED: &str = "ApiKeyCreated";
    pub const API_KEY_REVOKED: &str = "ApiKeyRevoked";
}

use uuid::Uuid;

use shared_types::Money;

#[derive(Debug, Clone)]
pub struct CreatePaymentIntentCommand {
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub amount: Money,
    pub idempotency_key: String,
    pub purpose: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub preferred_gateway_profile_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AuthorizePaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub payment_method_token_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub operator_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CapturePaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub amount: Option<Money>,
    pub principal_id: Uuid,
    pub role: String,
    pub operator_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct VoidPaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub operator_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RefundPaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub amount: Money,
    pub principal_id: Uuid,
    pub role: String,
    pub operator_id: Uuid,
}

/// Activate a routing policy for an operator (SRS UC-011, EVT-11/EVT-12).
/// Uses Maker/Checker: the activation must be approved before taking effect.
#[derive(Debug, Clone)]
pub struct ActivateRoutingPolicyCommand {
    pub routing_policy_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub operator_id: Uuid,
}

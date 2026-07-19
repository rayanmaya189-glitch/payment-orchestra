use uuid::Uuid;

use shared_types::Money;

#[derive(Debug, Clone)]
pub struct CreatePaymentIntentCommand {
    pub operator_id: Uuid,
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
}

#[derive(Debug, Clone)]
pub struct CapturePaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub amount: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct VoidPaymentIntentCommand {
    pub payment_intent_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RefundPaymentIntentCommand {
    pub payment_intent_id: Uuid,
    pub amount: Money,
}

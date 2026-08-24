//! Query type definitions for orchestration-service.

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GetPaymentIntentQuery {
    pub payment_intent_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListPaymentIntentsQuery {
    pub operator_id: Uuid,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct GetActiveRoutingPolicyQuery {
    pub operator_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetPaymentMethodTokenQuery {
    pub token_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListActiveTokensQuery {
    pub operator_id: Uuid,
}

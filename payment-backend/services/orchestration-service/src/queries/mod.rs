//! Query handlers for orchestration-service read models.
//! Queries return current state without side effects.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

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

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_payment_intent(&self, query: GetPaymentIntentQuery) -> Result<Option<PaymentIntent>, OrchestrationError>;
    async fn list_payment_intents(&self, query: ListPaymentIntentsQuery) -> Result<Vec<PaymentIntent>, OrchestrationError>;
    async fn get_active_routing_policy(&self, query: GetActiveRoutingPolicyQuery) -> Result<Option<RoutingPolicy>, OrchestrationError>;
    async fn get_payment_method_token(&self, query: GetPaymentMethodTokenQuery) -> Result<Option<PaymentMethodToken>, OrchestrationError>;
    async fn list_active_tokens(&self, query: ListActiveTokensQuery) -> Result<Vec<PaymentMethodToken>, OrchestrationError>;
}

pub struct OrchestrationQueryHandler<R: PaymentIntentRepository + RoutingPolicyRepository + PaymentMethodTokenRepository> {
    repo: R,
}

impl<R: PaymentIntentRepository + RoutingPolicyRepository + PaymentMethodTokenRepository> OrchestrationQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PaymentIntentRepository + RoutingPolicyRepository + PaymentMethodTokenRepository + Send + Sync> QueryHandler for OrchestrationQueryHandler<R> {
    async fn get_payment_intent(&self, query: GetPaymentIntentQuery) -> Result<Option<PaymentIntent>, OrchestrationError> {
        self.repo.load_payment_intent(query.payment_intent_id).await
    }

    async fn list_payment_intents(&self, query: ListPaymentIntentsQuery) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let all = self.repo.list_payment_intents_for_operator(query.operator_id).await?;
        let offset = query.offset as usize;
        let limit = query.limit as usize;
        Ok(all.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_active_routing_policy(&self, query: GetActiveRoutingPolicyQuery) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        self.repo.load_active_routing_policy(query.operator_id).await
    }

    async fn get_payment_method_token(&self, query: GetPaymentMethodTokenQuery) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        self.repo.load_payment_method_token(query.token_id).await
    }

    async fn list_active_tokens(&self, query: ListActiveTokensQuery) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        self.repo.find_active_tokens_for_operator(query.operator_id).await
    }
}

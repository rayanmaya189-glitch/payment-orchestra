//! Repository trait definitions for orchestration-service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

/// Repository for PaymentIntent aggregate (event-sourced).
#[async_trait]
pub trait PaymentIntentRepository: Send + Sync {
    async fn load_payment_intent(&self, id: Uuid) -> Result<Option<PaymentIntent>, OrchestrationError>;
    async fn save_payment_intent(&self, intent: &mut PaymentIntent) -> Result<(), OrchestrationError>;
    async fn list_payment_intents_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentIntent>, OrchestrationError>;
}

/// Repository for RoutingPolicy aggregate (CRUD + events).
#[async_trait]
pub trait RoutingPolicyRepository: Send + Sync {
    async fn load_active_routing_policy(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError>;
    async fn save_routing_policy(&self, policy: &RoutingPolicy) -> Result<(), OrchestrationError>;
    async fn load_routing_policy(&self, id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError>;
}

/// Repository for PaymentMethodToken aggregate (CRUD + events).
#[async_trait]
pub trait PaymentMethodTokenRepository: Send + Sync {
    async fn load_payment_method_token(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, OrchestrationError>;
    async fn save_payment_method_token(&self, token: &PaymentMethodToken) -> Result<(), OrchestrationError>;
    async fn find_active_tokens_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, OrchestrationError>;
}

/// Idempotency cache for deduplication.
#[async_trait]
pub trait IdempotencyCache: Send + Sync {
    async fn check_idempotency(&self, key: &str) -> Result<IdempotencyResult, OrchestrationError>;
    async fn store_idempotency(&self, key: &str, result: &serde_json::Value) -> Result<(), OrchestrationError>;
}

/// Active acquirer link with its connector identifier.
#[derive(Debug, Clone)]
pub struct AcquirerLinkInfo {
    pub link_id: Uuid,
    pub connector_id: String,
}

/// Provides active acquirer links for routing decisions.
#[async_trait]
pub trait AcquirerLinkProvider: Send + Sync {
    async fn list_active_acquirer_links(&self, operator_id: Uuid) -> Result<Vec<Uuid>, OrchestrationError>;
    async fn get_acquirer_link_connector(&self, link_id: Uuid) -> Result<String, OrchestrationError>;
}

/// Combined supertrait for convenience.
pub trait OrchestrationRepository:
    PaymentIntentRepository
    + RoutingPolicyRepository
    + PaymentMethodTokenRepository
    + IdempotencyCache
    + AcquirerLinkProvider
{}

impl<T> OrchestrationRepository for T where
    T: PaymentIntentRepository
        + RoutingPolicyRepository
        + PaymentMethodTokenRepository
        + IdempotencyCache
        + AcquirerLinkProvider
        + Send
        + Sync
{}

//! Repository interfaces and in-memory implementation for orchestration-service.
//! In production, these would be backed by SeaORM + PostgreSQL + Redis.

pub mod pg;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

// ─── Repository Traits ───────────────────────────────────────────────────────

#[async_trait]
pub trait PaymentIntentRepository: Send + Sync {
    async fn load_payment_intent(&self, id: Uuid) -> Result<Option<PaymentIntent>, OrchestrationError>;
    async fn save_payment_intent(&self, intent: &PaymentIntent) -> Result<(), OrchestrationError>;
    async fn list_payment_intents_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentIntent>, OrchestrationError>;
}

#[async_trait]
pub trait RoutingPolicyRepository: Send + Sync {
    async fn load_active_routing_policy(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError>;
    async fn save_routing_policy(&self, policy: &RoutingPolicy) -> Result<(), OrchestrationError>;
    async fn load_routing_policy(&self, id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError>;
}

#[async_trait]
pub trait PaymentMethodTokenRepository: Send + Sync {
    async fn load_payment_method_token(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, OrchestrationError>;
    async fn save_payment_method_token(&self, token: &PaymentMethodToken) -> Result<(), OrchestrationError>;
    async fn find_active_tokens_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, OrchestrationError>;
}

#[async_trait]
pub trait IdempotencyCache: Send + Sync {
    async fn check_idempotency(&self, key: &str) -> Result<IdempotencyResult, OrchestrationError>;
    async fn store_idempotency(&self, key: &str, result: &serde_json::Value) -> Result<(), OrchestrationError>;
}

#[async_trait]
pub trait AcquirerLinkProvider: Send + Sync {
    async fn list_active_acquirer_links(&self, operator_id: Uuid) -> Result<Vec<Uuid>, OrchestrationError>;
}

// ─── Combined Repository (for convenience) ───────────────────────────────────

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

// ─── In-Memory Repository Implementation (for testing) ───────────────────────

#[derive(Clone)]
pub struct InMemoryOrchestrationRepository {
    payment_intents: Arc<RwLock<HashMap<Uuid, PaymentIntent>>>,
    routing_policies: Arc<RwLock<HashMap<Uuid, RoutingPolicy>>>,
    tokens: Arc<RwLock<HashMap<Uuid, PaymentMethodToken>>>,
    idempotency_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    active_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>, // operator_id → policy_id
    active_links: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // operator_id → [link_ids]
}

impl InMemoryOrchestrationRepository {
    pub fn new() -> Self {
        Self {
            payment_intents: Arc::new(RwLock::new(HashMap::new())),
            routing_policies: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
            active_policies: Arc::new(RwLock::new(HashMap::new())),
            active_links: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Seed active acquirer links for testing
    pub async fn set_active_links(&self, operator_id: Uuid, link_ids: Vec<Uuid>) {
        let mut links = self.active_links.write().await;
        links.insert(operator_id, link_ids);
    }
}

#[async_trait]
impl PaymentIntentRepository for InMemoryOrchestrationRepository {
    async fn load_payment_intent(&self, id: Uuid) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let store = self.payment_intents.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_payment_intent(&self, intent: &PaymentIntent) -> Result<(), OrchestrationError> {
        let mut store = self.payment_intents.write().await;
        store.insert(intent.payment_intent_id, intent.clone());
        Ok(())
    }

    async fn list_payment_intents_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let store = self.payment_intents.read().await;
        Ok(store.values().filter(|pi| pi.operator_id == operator_id).cloned().collect())
    }
}

#[async_trait]
impl RoutingPolicyRepository for InMemoryOrchestrationRepository {
    async fn load_active_routing_policy(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let active = self.active_policies.read().await;
        if let Some(policy_id) = active.get(&operator_id) {
            let store = self.routing_policies.read().await;
            return Ok(store.get(policy_id).cloned());
        }
        Ok(None)
    }

    async fn save_routing_policy(&self, policy: &RoutingPolicy) -> Result<(), OrchestrationError> {
        let mut store = self.routing_policies.write().await;
        store.insert(policy.routing_policy_id, policy.clone());
        if policy.status == PolicyStatus::Active {
            let mut active = self.active_policies.write().await;
            active.insert(policy.operator_id, policy.routing_policy_id);
        }
        Ok(())
    }

    async fn load_routing_policy(&self, id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let store = self.routing_policies.read().await;
        Ok(store.get(&id).cloned())
    }
}

#[async_trait]
impl PaymentMethodTokenRepository for InMemoryOrchestrationRepository {
    async fn load_payment_method_token(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let store = self.tokens.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_payment_method_token(&self, token: &PaymentMethodToken) -> Result<(), OrchestrationError> {
        let mut store = self.tokens.write().await;
        store.insert(token.token_id, token.clone());
        Ok(())
    }

    async fn find_active_tokens_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let store = self.tokens.read().await;
        Ok(store.values()
            .filter(|t| t.operator_id == operator_id && t.token_status == TokenStatus::Active)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl IdempotencyCache for InMemoryOrchestrationRepository {
    async fn check_idempotency(&self, key: &str) -> Result<IdempotencyResult, OrchestrationError> {
        let cache = self.idempotency_cache.read().await;
        match cache.get(key) {
            Some(result) => Ok(IdempotencyResult::Duplicate(result.clone())),
            None => Ok(IdempotencyResult::New),
        }
    }

    async fn store_idempotency(&self, key: &str, result: &serde_json::Value) -> Result<(), OrchestrationError> {
        let mut cache = self.idempotency_cache.write().await;
        cache.insert(key.to_string(), result.clone());
        Ok(())
    }
}

#[async_trait]
impl AcquirerLinkProvider for InMemoryOrchestrationRepository {
    async fn list_active_acquirer_links(&self, operator_id: Uuid) -> Result<Vec<Uuid>, OrchestrationError> {
        let links = self.active_links.read().await;
        Ok(links.get(&operator_id).cloned().unwrap_or_default())
    }
}

//! In-memory implementations of orchestration repository traits.
//!
//! Fields are `pub(super)` to allow impl blocks in sibling modules
//! to access them for the in-memory backing stores.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

#[derive(Clone)]
pub struct InMemoryOrchestrationRepository {
    pub(super) payment_intents: Arc<RwLock<HashMap<Uuid, PaymentIntent>>>,
    pub(super) routing_policies: Arc<RwLock<HashMap<Uuid, RoutingPolicy>>>,
    pub(super) tokens: Arc<RwLock<HashMap<Uuid, PaymentMethodToken>>>,
    pub(super) idempotency_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub(super) active_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    pub(super) active_links: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
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

    /// Seed active acquirer links for testing.
    pub async fn set_active_links(&self, operator_id: Uuid, link_ids: Vec<Uuid>) {
        let mut links = self.active_links.write().await;
        links.insert(operator_id, link_ids);
    }
}

//! PostgreSQL-backed orchestration repositories using SeaORM + platform-db entities.
//!
//! Implements 3 persistent traits (PaymentIntentRepository, RoutingPolicyRepository,
//! PaymentMethodTokenRepository) and 2 in-memory traits (IdempotencyCache, AcquirerLinkProvider).

use std::collections::HashMap;
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod payment_intent;
pub mod routing_policy;
pub mod idempotency;
pub mod payment_method_token;
pub mod acquirer_link;

/// Combined PostgreSQL-backed orchestration repository.
///
/// Uses SeaORM for persistent entities and in-memory maps for
/// idempotency cache and acquirer link provider (no dedicated DB tables yet).
pub struct PostgresOrchestrationRepository {
    pub db: DatabaseConnection,
    pub(super) idempotency_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub(super) active_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    pub(super) active_links: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl PostgresOrchestrationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
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

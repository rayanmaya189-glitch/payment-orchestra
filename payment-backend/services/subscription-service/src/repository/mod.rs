//! Subscription Billing repository — BC-08

pub mod pg;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

// ---------------------------------------------------------------------------
// Repository trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError>;
    async fn save(&self, subscription: &Subscription) -> Result<(), SubscriptionError>;
    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError>;
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InMemorySubscriptionRepository {
    subscriptions: Arc<RwLock<HashMap<Uuid, Subscription>>>,
}

impl InMemorySubscriptionRepository {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SubscriptionRepository for InMemorySubscriptionRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError> {
        let map = self.subscriptions.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, subscription: &Subscription) -> Result<(), SubscriptionError> {
        let mut map = self.subscriptions.write().await;
        map.insert(subscription.subscription_id, subscription.clone());
        Ok(())
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        let map = self.subscriptions.read().await;
        let active: Vec<Subscription> = map
            .values()
            .filter(|s| {
                s.status == SubscriptionStatus::Active
                    || s.status == SubscriptionStatus::PastDue
            })
            .cloned()
            .collect();
        Ok(active)
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let map = self.subscriptions.read().await;
        let results: Vec<Subscription> = map
            .values()
            .filter(|s| s.customer_id == customer_id)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let map = self.subscriptions.read().await;
        let results: Vec<Subscription> = map
            .values()
            .filter(|s| s.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }
}

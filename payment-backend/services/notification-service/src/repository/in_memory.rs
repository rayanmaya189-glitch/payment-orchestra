//! In-memory notification and webhook repositories — BC-14

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::{NotificationRepository, WebhookRepository};

#[derive(Clone)]
pub struct InMemoryNotificationRepository {
    pub(super) notifications: Arc<RwLock<HashMap<Uuid, NotificationRequest>>>,
}

impl InMemoryNotificationRepository {
    pub fn new() -> Self {
        Self { notifications: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl NotificationRepository for InMemoryNotificationRepository {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, request: &NotificationRequest) -> Result<(), NotificationError> {
        let mut map = self.notifications.write().await;
        map.insert(request.notification_id, request.clone());
        Ok(())
    }

    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        let results: Vec<NotificationRequest> = map.values()
            .filter(|n| n.status == DeliveryStatus::Queued || n.status == DeliveryStatus::Failed)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        let results: Vec<NotificationRequest> = map.values()
            .filter(|n| n.status == DeliveryStatus::DeadLetter)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        let results: Vec<NotificationRequest> = map.values()
            .filter(|n| n.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }
}

#[derive(Clone)]
pub struct InMemoryWebhookRepository {
    pub(super) webhooks: Arc<RwLock<HashMap<Uuid, Webhook>>>,
}

impl InMemoryWebhookRepository {
    pub fn new() -> Self {
        Self { webhooks: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl WebhookRepository for InMemoryWebhookRepository {
    async fn save(&self, webhook: &Webhook) -> Result<(), NotificationError> {
        let mut map = self.webhooks.write().await;
        map.insert(webhook.webhook_id, webhook.clone());
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Webhook>, NotificationError> {
        let map = self.webhooks.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Webhook>, NotificationError> {
        let map = self.webhooks.read().await;
        let results: Vec<Webhook> = map.values()
            .filter(|w| w.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn delete(&self, id: Uuid) -> Result<(), NotificationError> {
        let mut map = self.webhooks.write().await;
        map.remove(&id);
        Ok(())
    }
}

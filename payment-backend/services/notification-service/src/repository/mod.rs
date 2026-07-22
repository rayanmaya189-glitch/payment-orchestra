//! Notification Service repository — BC-14

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
pub trait NotificationRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, NotificationError>;
    async fn save(&self, request: &NotificationRequest) -> Result<(), NotificationError>;
    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError>;
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InMemoryNotificationRepository {
    notifications: Arc<RwLock<HashMap<Uuid, NotificationRequest>>>,
}

impl InMemoryNotificationRepository {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(RwLock::new(HashMap::new())),
        }
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
        let results: Vec<NotificationRequest> = map
            .values()
            .filter(|n| n.status == DeliveryStatus::Queued || n.status == DeliveryStatus::Failed)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        let results: Vec<NotificationRequest> = map
            .values()
            .filter(|n| n.status == DeliveryStatus::DeadLetter)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        let map = self.notifications.read().await;
        let results: Vec<NotificationRequest> = map
            .values()
            .filter(|n| n.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }
}

//! Notification Service repository traits — BC-14

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, NotificationError>;
    async fn save(&self, request: &NotificationRequest) -> Result<(), NotificationError>;
    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError>;
}

#[async_trait]
pub trait WebhookRepository: Send + Sync {
    async fn save(&self, webhook: &Webhook) -> Result<(), NotificationError>;
    async fn load(&self, id: Uuid) -> Result<Option<Webhook>, NotificationError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Webhook>, NotificationError>;
    async fn delete(&self, id: Uuid) -> Result<(), NotificationError>;
}

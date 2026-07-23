//! Notification Service query handlers — BC-14

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_notification(&self, id: Uuid) -> Result<NotificationRequest, NotificationError>;
    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError>;
    async fn list_templates(&self) -> Vec<NotificationTemplate>;
}

pub struct NotificationQueryHandler<R: NotificationRepository> {
    repo: R,
}

impl<R: NotificationRepository> NotificationQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: NotificationRepository + Send + Sync> QueryHandler for NotificationQueryHandler<R> {
    async fn get_notification(&self, id: Uuid) -> Result<NotificationRequest, NotificationError> {
        self.repo
            .load(id)
            .await?
            .ok_or(NotificationError::NotFound(id))
    }

    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.repo.find_pending().await
    }

    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.repo.find_dead_letter().await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.repo.find_by_operator(operator_id).await
    }

    async fn list_templates(&self) -> Vec<NotificationTemplate> {
        default_templates()
    }
}

//! Notification Service API surface — BC-14

use crate::commands::*;
use crate::domain::{NotificationError, NotificationRequest, NotificationTemplate};
use crate::queries::*;
use uuid::Uuid;

pub mod grpc;

pub struct NotificationApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl NotificationApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    pub async fn send_notification(
        &self,
        cmd: SendNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        self.command_handler.send_notification(cmd).await
    }

    pub async fn mark_delivered(
        &self,
        cmd: MarkDeliveredCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        self.command_handler.mark_delivered(cmd).await
    }

    pub async fn mark_failed(
        &self,
        cmd: MarkFailedCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        self.command_handler.mark_failed(cmd).await
    }

    pub async fn retry_notification(
        &self,
        cmd: RetryNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        self.command_handler.retry_notification(cmd).await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_notification(&self, id: Uuid) -> Result<NotificationRequest, NotificationError> {
        self.query_handler.get_notification(id).await
    }

    pub async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.query_handler.find_pending().await
    }

    pub async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.query_handler.find_dead_letter().await
    }

    pub async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        self.query_handler.find_by_operator(operator_id).await
    }

    pub async fn list_templates(&self) -> Vec<NotificationTemplate> {
        self.query_handler.list_templates().await
    }
}

//! Notification Service command handlers — BC-14

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn send_notification(
        &self,
        cmd: SendNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError>;

    async fn mark_delivered(
        &self,
        cmd: MarkDeliveredCommand,
    ) -> Result<NotificationRequest, NotificationError>;

    async fn mark_failed(
        &self,
        cmd: MarkFailedCommand,
    ) -> Result<NotificationRequest, NotificationError>;

    async fn retry_notification(
        &self,
        cmd: RetryNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError>;
}

pub struct NotificationCommandHandler<R: NotificationRepository> {
    repo: R,
}

impl<R: NotificationRepository> NotificationCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: NotificationRepository + Send + Sync> CommandHandler for NotificationCommandHandler<R> {
    async fn send_notification(
        &self,
        cmd: SendNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        let templates = default_templates();
        if !templates.iter().any(|t| t.template_id == cmd.template_id) {
            return Err(NotificationError::TemplateMissing(cmd.template_id));
        }

        let notification = NotificationRequest::new(
            cmd.operator_id,
            cmd.channel,
            cmd.recipient,
            cmd.template_id,
            cmd.payload_json,
            cmd.subject,
        );

        self.repo.save(&notification).await?;
        Ok(notification)
    }

    async fn mark_delivered(
        &self,
        cmd: MarkDeliveredCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        let mut notification = self
            .repo
            .load(cmd.notification_id)
            .await?
            .ok_or(NotificationError::NotFound(cmd.notification_id))?;

        notification.mark_sent()?;
        self.repo.save(&notification).await?;
        Ok(notification)
    }

    async fn mark_failed(
        &self,
        cmd: MarkFailedCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        let mut notification = self
            .repo
            .load(cmd.notification_id)
            .await?
            .ok_or(NotificationError::NotFound(cmd.notification_id))?;

        notification.mark_failed(cmd.error)?;
        self.repo.save(&notification).await?;
        Ok(notification)
    }

    async fn retry_notification(
        &self,
        cmd: RetryNotificationCommand,
    ) -> Result<NotificationRequest, NotificationError> {
        let mut notification = self
            .repo
            .load(cmd.notification_id)
            .await?
            .ok_or(NotificationError::NotFound(cmd.notification_id))?;

        if !notification.status.can_retry() {
            return Err(match notification.status {
                DeliveryStatus::Sent => NotificationError::AlreadySent,
                _ => NotificationError::MaxRetriesExceeded,
            });
        }

        notification.reset_for_retry();
        self.repo.save(&notification).await?;
        Ok(notification)
    }
}

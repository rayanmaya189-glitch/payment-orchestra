use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::Notification;
use crate::domain::rules::*;
use crate::domain::value_objects::NotificationType;
use platform_error::PlatformError;

pub struct NotificationServiceImpl {
    repo: Box<dyn NotificationRepository>,
    db: DatabaseConnection,
}

impl NotificationServiceImpl {
    pub fn new(repo: Box<dyn NotificationRepository>, db: DatabaseConnection) -> Self {
        Self { repo, db }
    }

    async fn dispatch_notification(&self, notification: &mut Notification) -> Result<(), PlatformError> {
        match notification.notification_type {
            NotificationType::Email => {
                let provider = crate::infrastructure::adapters::MockEmailProvider::new();
                match provider.send_email(&notification.recipient, notification.subject.as_deref().unwrap_or(""), &notification.body).await {
                    Ok(msg_id) => notification.mark_sent(msg_id),
                    Err(e) => notification.mark_failed(e.to_string()),
                }
            }
            NotificationType::Sms => {
                notification.mark_sent(format!("sms_{}", Uuid::now_v7()));
            }
            NotificationType::Webhook => {
                notification.mark_sent(format!("wh_{}", Uuid::now_v7()));
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send(&self, cmd: SendNotificationCommand) -> Result<Uuid, PlatformError>;
    async fn retry(&self, cmd: RetryNotificationCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<Notification, PlatformError>;
}

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn send(&self, cmd: SendNotificationCommand) -> Result<Uuid, PlatformError> {
        let mut notification = Notification::new(
            cmd.operator_id,
            NotificationType::from_str(&cmd.notification_type),
            cmd.recipient, cmd.subject, cmd.body,
        );
        notification.template_id = cmd.template_id;
        notification.template_data = cmd.template_data;
        notification.render_template();
        self.dispatch_notification(&mut notification).await?;
        self.repo.save(&notification).await?;
        Ok(notification.notification_id)
    }

    async fn retry(&self, cmd: RetryNotificationCommand) -> Result<(), PlatformError> {
        let mut notification = self.repo.find_by_id(cmd.notification_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "notification".into(), id: cmd.notification_id })?;
        if !notification.can_retry() {
            return Err(PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: notification.status.as_str().to_string(), command: "retry".to_string(),
            }));
        }
        self.dispatch_notification(&mut notification).await?;
        self.repo.save(&notification).await
    }

    async fn get(&self, id: Uuid) -> Result<Notification, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "notification".into(), id })
    }
}

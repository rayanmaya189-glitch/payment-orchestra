use async_trait::async_trait; use uuid::Uuid;
use crate::domain::aggregates::Notification;
use crate::domain::value_objects::NotificationType;
use platform_error::PlatformError;

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_notification(&self, cmd: SendNotificationCommand) -> Result<NotificationResponse, PlatformError>;
    async fn get_notification(&self, id: Uuid) -> Result<NotificationResponse, PlatformError>;
}

pub struct NotificationServiceImpl { db: sea_orm::DatabaseConnection }
impl NotificationServiceImpl { pub fn new(db: sea_orm::DatabaseConnection) -> Self { Self { db } } }

pub struct SendNotificationCommand { pub operator_id: Uuid, pub notification_type: String, pub recipient: String, pub subject: Option<String>, pub body: String }
#[derive(Debug, Clone)]
pub struct NotificationResponse { pub notification_id: Uuid, pub status: String, pub notification_type: String, pub recipient: String }

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn send_notification(&self, cmd: SendNotificationCommand) -> Result<NotificationResponse, PlatformError> {
        let ntype = NotificationType::from_str(&cmd.notification_type);
        let mut notification = Notification::new(cmd.operator_id, ntype, cmd.recipient, cmd.subject, cmd.body);
        notification.mark_sent();
        Ok(notification_to_response(&notification))
    }
    async fn get_notification(&self, id: Uuid) -> Result<NotificationResponse, PlatformError> { Err(PlatformError::NotFound { resource: "Notification".into(), id }) }
}

fn notification_to_response(n: &Notification) -> NotificationResponse { NotificationResponse { notification_id: n.notification_id, status: n.status.as_str().to_string(), notification_type: n.notification_type.as_str().to_string(), recipient: n.recipient.clone() } }

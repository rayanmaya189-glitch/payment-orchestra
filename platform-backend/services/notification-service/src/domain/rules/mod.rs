use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Notification;
use platform_error::PlatformError;

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, PlatformError>;
    async fn save(&self, notification: &Notification) -> Result<(), PlatformError>;
    async fn find_pending_retries(&self) -> Result<Vec<Notification>, PlatformError>;
}

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<String, PlatformError>;
}

#[async_trait]
pub trait SmsProvider: Send + Sync {
    async fn send_sms(&self, to: &str, body: &str) -> Result<String, PlatformError>;
}

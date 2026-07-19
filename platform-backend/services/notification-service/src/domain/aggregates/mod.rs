use chrono::{DateTime, Utc}; use uuid::Uuid;
use crate::domain::value_objects::{NotificationStatus, NotificationType};

#[derive(Debug, Clone)]
pub struct Notification {
    pub notification_id: Uuid, pub operator_id: Uuid, pub notification_type: NotificationType,
    pub status: NotificationStatus, pub recipient: String, pub subject: Option<String>,
    pub body: String, pub template_id: Option<String>, pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>, pub sent_at: Option<DateTime<Utc>>,
}
impl Notification {
    pub fn new(operator_id: Uuid, notification_type: NotificationType, recipient: String, subject: Option<String>, body: String) -> Self {
        Self { notification_id: Uuid::now_v7(), operator_id, notification_type, status: NotificationStatus::Pending, recipient, subject, body, template_id: None, metadata: None, created_at: Utc::now(), sent_at: None }
    }
    pub fn mark_sent(&mut self) { self.status = NotificationStatus::Sent; self.sent_at = Some(Utc::now()); }
    pub fn mark_failed(&mut self) { self.status = NotificationStatus::Failed; }
}

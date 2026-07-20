use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{NotificationType, NotificationStatus};

#[derive(Debug, Clone)]
pub struct Notification {
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub notification_type: NotificationType,
    pub status: NotificationStatus,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub template_id: Option<String>,
    pub template_data: Option<serde_json::Value>,
    pub provider_message_id: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(operator_id: Uuid, notification_type: NotificationType, recipient: String, subject: Option<String>, body: String) -> Self {
        let now = Utc::now();
        Self {
            notification_id: Uuid::now_v7(), operator_id, notification_type,
            status: NotificationStatus::Pending, recipient, subject, body,
            template_id: None, template_data: None, provider_message_id: None,
            retry_count: 0, max_retries: 3, sent_at: None, delivered_at: None,
            failed_at: None, error_message: None, created_at: now, updated_at: now,
        }
    }

    pub fn mark_sent(&mut self, provider_message_id: String) {
        self.status = NotificationStatus::Sent;
        self.provider_message_id = Some(provider_message_id);
        self.sent_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_delivered(&mut self) {
        self.status = NotificationStatus::Delivered;
        self.delivered_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.retry_count += 1;
        self.error_message = Some(error);
        if self.retry_count >= self.max_retries {
            self.status = NotificationStatus::Failed;
            self.failed_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
    }

    pub fn can_retry(&self) -> bool {
        self.status == NotificationStatus::Failed && self.retry_count < self.max_retries
    }

    pub fn render_template(&mut self) {
        if let (Some(ref template), Some(ref data)) = (&self.template_id, &self.template_data) {
            if let Some(obj) = data.as_object() {
                let mut rendered = self.body.clone();
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        rendered = rendered.replace(&format!("{{{{{key}}}}}"), val_str);
                    }
                }
                self.body = rendered;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_notification() {
        let n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), Some("Subject".into()), "Body".into());
        assert_eq!(n.status, NotificationStatus::Pending);
        assert_eq!(n.retry_count, 0);
    }

    #[test]
    fn test_mark_sent() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), None, "Body".into());
        n.mark_sent("msg_123".into());
        assert_eq!(n.status, NotificationStatus::Sent);
        assert_eq!(n.provider_message_id, Some("msg_123".into()));
    }

    #[test]
    fn test_mark_failed_increments_retry() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), None, "Body".into());
        n.mark_failed("timeout".into());
        assert_eq!(n.retry_count, 1);
        assert_eq!(n.status, NotificationStatus::Pending); // not failed yet
    }

    #[test]
    fn test_max_retries_marks_failed() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), None, "Body".into());
        n.max_retries = 2;
        n.mark_failed("fail 1".into());
        n.mark_failed("fail 2".into());
        assert_eq!(n.status, NotificationStatus::Failed);
    }

    #[test]
    fn test_render_template() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), None, "Hello {{name}}!".into());
        n.template_data = Some(serde_json::json!({"name": "World"}));
        n.render_template();
        assert_eq!(n.body, "Hello World!");
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_notification_is_pending() {
        let n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), Some("Subject".into()), "Body".into());
        assert_eq!(n.status, NotificationStatus::Pending);
        assert!(n.sent_at.is_none());
    }

    #[test]
    fn test_mark_sent() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Email, "test@example.com".into(), None, "Body".into());
        n.mark_sent();
        assert_eq!(n.status, NotificationStatus::Sent);
        assert!(n.sent_at.is_some());
    }

    #[test]
    fn test_mark_failed() {
        let mut n = Notification::new(Uuid::now_v7(), NotificationType::Sms, "+971501234567".into(), None, "Code: 1234".into());
        n.mark_failed();
        assert_eq!(n.status, NotificationStatus::Failed);
    }

    #[test]
    fn test_notification_type_values() {
        assert_eq!(NotificationType::Email.as_str(), "email");
        assert_eq!(NotificationType::Sms.as_str(), "sms");
        assert_eq!(NotificationType::Push.as_str(), "push");
    }
}

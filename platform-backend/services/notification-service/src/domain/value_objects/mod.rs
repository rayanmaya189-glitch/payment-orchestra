use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationType {
    Email,
    Sms,
    Webhook,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Sms => "sms",
            Self::Webhook => "webhook",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "sms" => Self::Sms,
            "webhook" => Self::Webhook,
            _ => Self::Email,
        }
    }
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    DeadLetter,
}

impl NotificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::DeadLetter => "dead_letter",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "sent" => Self::Sent,
            "delivered" => Self::Delivered,
            "failed" => Self::Failed,
            "dead_letter" => Self::DeadLetter,
            _ => Self::Pending,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Delivered | Self::DeadLetter)
    }

    pub fn can_retry(&self) -> bool {
        matches!(self, Self::Failed | Self::Pending)
    }
}

impl fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Rendered template with subject and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedTemplate {
    pub subject: Option<String>,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_roundtrip() {
        for variant in [NotificationType::Email, NotificationType::Sms, NotificationType::Webhook] {
            let s = variant.as_str();
            assert_eq!(NotificationType::from_str(s), variant);
        }
    }

    #[test]
    fn test_notification_status_roundtrip() {
        for variant in [
            NotificationStatus::Pending,
            NotificationStatus::Sent,
            NotificationStatus::Delivered,
            NotificationStatus::Failed,
            NotificationStatus::DeadLetter,
        ] {
            let s = variant.as_str();
            assert_eq!(NotificationStatus::from_str(s), variant);
        }
    }

    #[test]
    fn test_status_terminal() {
        assert!(NotificationStatus::Delivered.is_terminal());
        assert!(NotificationStatus::DeadLetter.is_terminal());
        assert!(!NotificationStatus::Pending.is_terminal());
        assert!(!NotificationStatus::Failed.is_terminal());
    }

    #[test]
    fn test_status_can_retry() {
        assert!(NotificationStatus::Failed.can_retry());
        assert!(NotificationStatus::Pending.can_retry());
        assert!(!NotificationStatus::Delivered.can_retry());
        assert!(!NotificationStatus::DeadLetter.can_retry());
    }

    #[test]
    fn test_notification_type_display() {
        assert_eq!(NotificationType::Email.to_string(), "email");
        assert_eq!(NotificationType::Sms.to_string(), "sms");
    }

    #[test]
    fn test_notification_type_from_str_unknown_defaults_to_email() {
        assert_eq!(NotificationType::from_str("unknown"), NotificationType::Email);
    }

    #[test]
    fn test_notification_status_from_str_unknown_defaults_to_pending() {
        assert_eq!(NotificationStatus::from_str("bogus"), NotificationStatus::Pending);
    }
}

//! Notification delivery channel types — Email, SMS, Webhook.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    Webhook,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::Webhook => write!(f, "webhook"),
        }
    }
}

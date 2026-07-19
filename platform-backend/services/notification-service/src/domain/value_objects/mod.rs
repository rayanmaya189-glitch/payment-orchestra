#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType { Email, Sms, Push }
impl NotificationType {
    pub fn as_str(&self) -> &'static str { match self { Self::Email => "email", Self::Sms => "sms", Self::Push => "push" } }
    pub fn from_str(s: &str) -> Self { match s { "email" => Self::Email, "sms" => Self::Sms, "push" => Self::Push, _ => Self::Email } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus { Pending, Sent, Failed }
impl NotificationStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Sent => "sent", Self::Failed => "failed" } }
}

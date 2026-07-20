#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType { Email, Sms, Push }
impl NotificationType {
    pub fn as_str(&self) -> &'static str { match self { Self::Email => "email", Self::Sms => "sms", Self::Push => "push" } }
    pub fn from_str(s: &str) -> Result<Self, &'static str> { match s { "email" => Ok(Self::Email), "sms" => Ok(Self::Sms), "push" => Ok(Self::Push), _ => Err("unknown notification type") } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus { Pending, Sent, Failed }
impl NotificationStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Sent => "sent", Self::Failed => "failed" } }
}

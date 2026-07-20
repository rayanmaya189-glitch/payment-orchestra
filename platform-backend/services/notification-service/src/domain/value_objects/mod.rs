#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType { Email, Sms, Webhook }
impl NotificationType {
    pub fn as_str(&self) -> &'static str { match self { Self::Email => "email", Self::Sms => "sms", Self::Webhook => "webhook" } }
    pub fn from_str(s: &str) -> Self { match s { "sms" => Self::Sms, "webhook" => Self::Webhook, _ => Self::Email } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationStatus { Pending, Sent, Delivered, Failed }
impl NotificationStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Sent => "sent", Self::Delivered => "delivered", Self::Failed => "failed" } }
    pub fn from_str(s: &str) -> Self { match s { "sent" => Self::Sent, "delivered" => Self::Delivered, "failed" => Self::Failed, _ => Self::Pending } }
}

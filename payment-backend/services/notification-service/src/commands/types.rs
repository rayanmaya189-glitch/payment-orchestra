//! Command types for BC-14 Notification Service

use uuid::Uuid;
use crate::domain::*;

/// Send a new notification via a specific channel.
pub struct SendNotificationCommand {
    pub operator_id: Uuid,
    pub channel: NotificationChannel,
    pub recipient: String,
    pub template_id: String,
    pub payload_json: String,
    pub subject: Option<String>,
}

/// Mark a notification as successfully delivered.
pub struct MarkDeliveredCommand {
    pub notification_id: Uuid,
}

/// Mark a notification as failed (with optional retry).
pub struct MarkFailedCommand {
    pub notification_id: Uuid,
    pub error: String,
}

/// Retry a failed notification.
pub struct RetryNotificationCommand {
    pub notification_id: Uuid,
}

//! Notification Service domain events — BC-14

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEvent {
    RequestCreated(NotificationRequestCreated),
    Delivered(NotificationDelivered),
    DeliveryFailed(NotificationDeliveryFailed),
    DeadLettered(NotificationDeadLettered),
}

pub const EVENT_TYPE_REQUEST_CREATED: &str = "notification.request_created";
pub const EVENT_TYPE_DELIVERED: &str = "notification.delivered";
pub const EVENT_TYPE_DELIVERY_FAILED: &str = "notification.delivery_failed";
pub const EVENT_TYPE_DEAD_LETTERED: &str = "notification.dead_lettered";

impl NotificationEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::RequestCreated(_) => EVENT_TYPE_REQUEST_CREATED,
            Self::Delivered(_) => EVENT_TYPE_DELIVERED,
            Self::DeliveryFailed(_) => EVENT_TYPE_DELIVERY_FAILED,
            Self::DeadLettered(_) => EVENT_TYPE_DEAD_LETTERED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequestCreated {
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub channel: String,
    pub recipient: String,
    pub template_id: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDelivered {
    pub notification_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDeliveryFailed {
    pub notification_id: Uuid,
    pub retry_count: i32,
    pub error: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDeadLettered {
    pub notification_id: Uuid,
    pub retry_count: i32,
    pub last_error: String,
    pub occurred_at: DateTime<Utc>,
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebhookEventType {
    PaymentCreated,
    PaymentAuthorized,
    PaymentCaptured,
    PaymentFailed,
    PaymentRefunded,
    PaymentVoided,
    SettlementMatched,
    SettlementUnmatched,
    ChargebackReceived,
    ChargebackResolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub subscription_id: Uuid,
    pub url: String,
    pub event_types: Vec<WebhookEventType>,
    pub secret: String,
    pub status: SubscriptionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub delivery_id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: String,
    pub status: DeliveryStatus,
    pub attempt_count: u32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub response_status_code: Option<u16>,
    pub response_body: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    PermanentlyFailed,
}

//! Notification Service domain model — BC-14
//!
//! At-least-once delivery of email/SMS/webhook notifications.
//! Subscribes to domain events and dispatches via configured providers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// NotificationChannel
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// DeliveryStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// Notification queued, awaiting delivery.
    Queued,
    /// Successfully delivered.
    Sent,
    /// Delivery failed, retry pending.
    Failed,
    /// Max retries exceeded, moved to dead letter.
    DeadLetter,
}

impl DeliveryStatus {
    pub fn can_retry(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Sent => write!(f, "sent"),
            Self::Failed => write!(f, "failed"),
            Self::DeadLetter => write!(f, "dead_letter"),
        }
    }
}

// ---------------------------------------------------------------------------
// NotificationRequest aggregate
// ---------------------------------------------------------------------------

/// A single notification request with delivery tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub channel: NotificationChannel,
    pub recipient: String,
    pub template_id: String,
    pub payload_json: String,
    pub subject: Option<String>,
    pub status: DeliveryStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl NotificationRequest {
    /// Create a new notification request in `Queued` status.
    pub fn new(
        operator_id: Uuid,
        channel: NotificationChannel,
        recipient: String,
        template_id: String,
        payload_json: String,
        subject: Option<String>,
    ) -> Self {
        Self {
            notification_id: Uuid::now_v7(),
            operator_id,
            channel,
            recipient,
            template_id,
            payload_json,
            subject,
            status: DeliveryStatus::Queued,
            retry_count: 0,
            max_retries: 3,
            created_at: Utc::now(),
            sent_at: None,
            last_error: None,
        }
    }

    /// Mark the notification as successfully sent.
    pub fn mark_sent(&mut self) -> Result<(), NotificationError> {
        if self.status == DeliveryStatus::Sent {
            return Err(NotificationError::AlreadySent);
        }
        self.status = DeliveryStatus::Sent;
        self.sent_at = Some(Utc::now());
        Ok(())
    }

    /// Mark delivery as failed. Retries if retries remain, otherwise dead letter.
    pub fn mark_failed(&mut self, error: String) -> Result<(), NotificationError> {
        self.retry_count += 1;
        self.last_error = Some(error);

        if self.retry_count >= self.max_retries {
            self.status = DeliveryStatus::DeadLetter;
        } else {
            self.status = DeliveryStatus::Failed;
        }
        Ok(())
    }

    /// Reset status for retry.
    pub fn reset_for_retry(&mut self) {
        self.status = DeliveryStatus::Queued;
    }
}

// ---------------------------------------------------------------------------
// Template (value object)
// ---------------------------------------------------------------------------

/// A notification template with subject and body. Supports variable substitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub template_id: String,
    pub channel: NotificationChannel,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub description: String,
}

impl NotificationTemplate {
    /// Render a template by substituting variables in `{{var_name}}` format.
    pub fn render(&self, variables: &std::collections::HashMap<String, String>) -> RenderedMessage {
        let mut subject = self.subject_template.clone();
        let mut body = self.body_template.clone();

        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            if let Some(ref mut s) = subject {
                *s = s.replace(&placeholder, value);
            }
            body = body.replace(&placeholder, value);
        }

        RenderedMessage { subject, body }
    }
}

/// A fully rendered notification message.
#[derive(Debug, Clone)]
pub struct RenderedMessage {
    pub subject: Option<String>,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Default templates
// ---------------------------------------------------------------------------

pub fn default_templates() -> Vec<NotificationTemplate> {
    vec![
        NotificationTemplate {
            template_id: "payment_failed".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Payment Failed - {{payment_intent_id}}".into()),
            body_template: "Payment {{payment_intent_id}} failed on all acquirers.\nAmount: {{amount}}\nOperator: {{operator_name}}\n\nPlease review the transaction in the dashboard.".into(),
            description: "Payment failed on all acquirers".into(),
        },
        NotificationTemplate {
            template_id: "settlement_unmatched".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Unmatched Settlement - {{batch_id}}".into()),
            body_template: "Settlement record {{transaction_id}} in batch {{batch_id}} is unmatched.\nAmount: {{amount}}\n\nPlease review in the reconciliation dashboard.".into(),
            description: "Unmatched settlement record detected".into(),
        },
        NotificationTemplate {
            template_id: "chargeback_received".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Chargeback Received - {{payment_intent_id}}".into()),
            body_template: "A chargeback has been received for payment {{payment_intent_id}}.\nReason: {{reason_code}}\nAmount: {{amount}}\nRepresentment deadline: {{deadline}}\n\nPlease review in the dispute dashboard.".into(),
            description: "New chargeback received".into(),
        },
        NotificationTemplate {
            template_id: "subscription_failed".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] Subscription Payment Failed".into()),
            body_template: "Your subscription payment of {{amount}} has failed.\nSubscription: {{subscription_id}}\nPlease update your payment method to avoid interruption.".into(),
            description: "Subscription renewal payment failed".into(),
        },
        NotificationTemplate {
            template_id: "api_key_expiring".into(),
            channel: NotificationChannel::Email,
            subject_template: Some("[Payment Orchestra] API Key Expiring - {{key_name}}".into()),
            body_template: "Your API key '{{key_name}}' is expiring in {{days_remaining}} days.\nPlease rotate your key before it expires.".into(),
            description: "API key is expiring soon".into(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum NotificationError {
    #[error("Notification not found: {0}")]
    NotFound(Uuid),
    #[error("Notification already sent")]
    AlreadySent,
    #[error("Notification template not found: {0}")]
    TemplateMissing(String),
    #[error("Email provider unavailable")]
    EmailProviderUnavailable,
    #[error("SMS provider unavailable")]
    SmsProviderUnavailable,
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}

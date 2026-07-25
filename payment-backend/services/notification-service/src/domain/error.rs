//! Notification domain errors.

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Error)]
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
    #[error("Webhook not found: {0}")]
    WebhookNotFound(Uuid),
}

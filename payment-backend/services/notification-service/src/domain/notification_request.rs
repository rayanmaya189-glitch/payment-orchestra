//! NotificationRequest aggregate — at-least-once delivery tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::delivery_status::DeliveryStatus;
use super::error::NotificationError;

/// A single notification request with delivery tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub channel: super::channel::NotificationChannel,
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
        channel: super::channel::NotificationChannel,
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

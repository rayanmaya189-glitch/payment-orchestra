use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::application::commands::*;
use crate::domain::aggregates::Notification;
use crate::domain::rules::*;
use crate::domain::value_objects::{NotificationStatus, NotificationType};
use platform_error::PlatformError;

// ==================== Service Trait ====================

#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Create and dispatch a notification. Validates input, renders template, dispatches via provider.
    async fn send(&self, cmd: SendNotificationCommand) -> Result<Uuid, PlatformError>;

    /// Retry a single failed notification.
    async fn retry(&self, cmd: RetryNotificationCommand) -> Result<(), PlatformError>;

    /// Retry all failed notifications (batch operation).
    async fn batch_retry(&self, cmd: BatchRetryCommand) -> Result<BatchRetryResult, PlatformError>;

    /// Get a notification by ID.
    async fn get(&self, id: Uuid) -> Result<Notification, PlatformError>;

    /// Mark a notification as delivered (e.g., from delivery webhook).
    async fn mark_delivered(&self, id: Uuid) -> Result<(), PlatformError>;

    /// List notifications for an operator.
    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Notification>, PlatformError>;

    /// Get retryable notifications count.
    async fn count_retryable(&self) -> Result<i64, PlatformError>;
}

#[derive(Debug, Clone)]
pub struct BatchRetryResult {
    pub total_retryable: usize,
    pub retried: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

// ==================== Service Implementation ====================

pub struct NotificationServiceImpl {
    repo: Box<dyn NotificationRepository>,
    email_provider: Box<dyn EmailProvider>,
    sms_provider: Box<dyn SmsProvider>,
    webhook_provider: Box<dyn WebhookProvider>,
    db: DatabaseConnection,
}

impl NotificationServiceImpl {
    pub fn new(
        repo: Box<dyn NotificationRepository>,
        email_provider: Box<dyn EmailProvider>,
        sms_provider: Box<dyn SmsProvider>,
        webhook_provider: Box<dyn WebhookProvider>,
        db: DatabaseConnection,
    ) -> Self {
        Self { repo, email_provider, sms_provider, webhook_provider, db: db }
    }

    /// Dispatch a notification to the appropriate provider based on type.
    ///
    /// Updates the notification in-place with dispatch outcome.
    async fn dispatch_notification(
        &self,
        notification: &mut Notification,
    ) -> Result<(), PlatformError> {
        let result = match notification.notification_type {
            NotificationType::Email => {
                self.email_provider
                    .send_email(
                        &notification.recipient,
                        notification.subject.as_deref().unwrap_or(""),
                        &notification.body,
                    )
                    .await
            }
            NotificationType::Sms => {
                self.sms_provider
                    .send_sms(&notification.recipient, &notification.body)
                    .await
            }
            NotificationType::Webhook => {
                let payload = serde_json::json!({
                    "notification_id": notification.notification_id.to_string(),
                    "type": notification.notification_type.as_str(),
                    "recipient": notification.recipient,
                    "subject": notification.subject,
                    "body": notification.body,
                });
                self.webhook_provider
                    .send_webhook(&notification.recipient, &payload)
                    .await
            }
        };

        match result {
            Ok(provider_result) => {
                notification.mark_sent(provider_result.message_id);
                Ok(())
            }
            Err(provider_error) => {
                notification.mark_failed(provider_error.to_string());
                // Don't return error — the notification is persisted with Failed status
                // and will be retried by the batch retry job.
                Ok(())
            }
        }
    }
}

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn send(&self, cmd: SendNotificationCommand) -> Result<Uuid, PlatformError> {
        let mut notification = Notification::new(
            cmd.operator_id,
            NotificationType::from_str(&cmd.notification_type),
            cmd.recipient,
            cmd.subject,
            cmd.body,
        );

        notification.template_id = cmd.template_id;
        notification.template_data = cmd.template_data;

        // Validate before rendering
        notification.validate().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::MissingField(e.to_string()))
        })?;

        // Render template
        notification.render_template().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::MissingField(e.to_string()))
        })?;

        // Dispatch
        self.dispatch_notification(&mut notification).await?;

        // Persist
        self.repo.save(&notification).await?;

        tracing::info!(
            notification_id = %notification.notification_id,
            status = %notification.status.as_str(),
            notification_type = %notification.notification_type.as_str(),
            "Notification dispatched"
        );

        Ok(notification.notification_id)
    }

    async fn retry(&self, cmd: RetryNotificationCommand) -> Result<(), PlatformError> {
        let mut notification = self
            .repo
            .find_by_id(cmd.notification_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "notification".into(),
                id: cmd.notification_id,
            })?;

        notification.prepare_retry().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: notification.status.as_str().to_string(),
                command: format!("retry: {e}"),
            })
        })?;

        self.dispatch_notification(&mut notification).await?;
        self.repo.save(&notification).await
    }

    async fn batch_retry(&self, cmd: BatchRetryCommand) -> Result<BatchRetryResult, PlatformError> {
        let retryable = self.repo.find_retryable().await?;
        let limit = cmd.limit.unwrap_or(100) as usize;
        let total = retryable.len().min(limit);

        let mut retried = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for mut notification in retryable.into_iter().take(limit) {
            if notification.prepare_retry().is_err() {
                failed += 1;
                errors.push(format!(
                    "notification {}: invalid state for retry",
                    notification.notification_id
                ));
                continue;
            }

            match self.dispatch_notification(&mut notification).await {
                Ok(()) => {
                    if let Err(e) = self.repo.save(&notification).await {
                        failed += 1;
                        errors.push(format!(
                            "notification {}: save failed: {e}",
                            notification.notification_id
                        ));
                    } else {
                        retried += 1;
                    }
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!(
                        "notification {}: dispatch failed: {e}",
                        notification.notification_id
                    ));
                }
            }
        }

        tracing::info!(
            total_retryable = total,
            retried = retried,
            failed = failed,
            "Batch retry completed"
        );

        Ok(BatchRetryResult {
            total_retryable: total,
            retried,
            failed,
            errors,
        })
    }

    async fn get(&self, id: Uuid) -> Result<Notification, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound {
            resource: "notification".into(),
            id,
        })
    }

    async fn mark_delivered(&self, id: Uuid) -> Result<(), PlatformError> {
        let mut notification = self.repo.find_by_id(id).await?.ok_or_else(|| {
            PlatformError::NotFound { resource: "notification".into(), id }
        })?;

        if notification.status == NotificationStatus::Sent {
            notification.mark_delivered();
            self.repo.save(&notification).await?;
        }
        // If already delivered or in other states, idempotent no-op.
        Ok(())
    }

    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Notification>, PlatformError> {
        self.repo.find_by_operator(operator_id, limit, offset).await
    }

    async fn count_retryable(&self) -> Result<i64, PlatformError> {
        let retryable = self.repo.find_retryable().await?;
        Ok(retryable.len() as i64)
    }
}

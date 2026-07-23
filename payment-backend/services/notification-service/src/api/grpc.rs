//! gRPC service implementation for notification-service (BC-14).
//! Translates between protobuf types and domain types for notification delivery.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{DeliveryStatus, NotificationChannel, NotificationError};
use crate::queries::QueryHandler;
use platform_proto::common::Timestamp;
use platform_proto::notification::notification_service_server::NotificationService;
use platform_proto::notification::*;

pub struct NotificationGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> NotificationGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> NotificationService for NotificationGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn send_email(
        &self,
        request: Request<SendEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        // Use "direct_email" template for raw content sends
        let cmd = commands::SendNotificationCommand {
            operator_id,
            channel: NotificationChannel::Email,
            recipient: req.to,
            template_id: "direct_email".into(),
            payload_json: serde_json::json!({
                "body": req.body_html,
                "body_text": req.body_text,
            }).to_string(),
            subject: if req.subject.is_empty() { None } else { Some(req.subject) },
        };

        match self.commands.send_notification(cmd).await {
            Ok(notification) => {
                Ok(Response::new(SendEmailResponse {
                    notification_id: notification.notification_id.to_string(),
                    status: notification.status.to_string(),
                }))
            }
            Err(e) => Err(notification_error_to_status(e)),
        }
    }

    async fn send_sms(
        &self,
        request: Request<SendSmsRequest>,
    ) -> Result<Response<SendSmsResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let cmd = commands::SendNotificationCommand {
            operator_id,
            channel: NotificationChannel::Sms,
            recipient: req.to,
            template_id: "direct_sms".into(),
            payload_json: serde_json::json!({
                "body": req.message,
            }).to_string(),
            subject: None,
        };

        match self.commands.send_notification(cmd).await {
            Ok(notification) => {
                Ok(Response::new(SendSmsResponse {
                    notification_id: notification.notification_id.to_string(),
                    status: notification.status.to_string(),
                }))
            }
            Err(e) => Err(notification_error_to_status(e)),
        }
    }

    async fn get_delivery_status(
        &self,
        request: Request<GetDeliveryStatusRequest>,
    ) -> Result<Response<GetDeliveryStatusResponse>, Status> {
        let req = request.into_inner();
        let notification_id = parse_uuid(&req.notification_id, "notification_id")?;

        match self.queries.get_notification(notification_id).await {
            Ok(notification) => {
                let next_retry = if notification.status == DeliveryStatus::Failed {
                    Some(Timestamp {
                        unix_ms: (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp_millis(),
                    })
                } else {
                    None
                };

                Ok(Response::new(GetDeliveryStatusResponse {
                    status: notification.status.to_string(),
                    attempt_count: notification.retry_count,
                    last_attempted_at: notification.sent_at.map(|dt| Timestamp {
                        unix_ms: dt.timestamp_millis(),
                    }),
                    next_retry_at: next_retry,
                }))
            }
            Err(e) => Err(notification_error_to_status(e)),
        }
    }

    async fn register_webhook(
        &self,
        request: Request<RegisterWebhookRequest>,
    ) -> Result<Response<RegisterWebhookResponse>, Status> {
        let _req = request.into_inner();
        // Webhook management is not yet implemented in the domain model.
        // Return a placeholder response.
        Ok(Response::new(RegisterWebhookResponse {
            webhook_id: Uuid::now_v7().to_string(),
            secret: "placeholder_secret".into(),
            status: "active".into(),
        }))
    }

    async fn list_webhooks(
        &self,
        _request: Request<ListWebhooksRequest>,
    ) -> Result<Response<ListWebhooksResponse>, Status> {
        // Webhook management not yet implemented — return empty list.
        Ok(Response::new(ListWebhooksResponse {
            webhooks: Vec::new(),
        }))
    }

    async fn delete_webhook(
        &self,
        request: Request<DeleteWebhookRequest>,
    ) -> Result<Response<DeleteWebhookResponse>, Status> {
        let _req = request.into_inner();
        // Webhook management not yet implemented — return success.
        Ok(Response::new(DeleteWebhookResponse {
            deleted: true,
        }))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn notification_error_to_status(e: NotificationError) -> Status {
    match e {
        NotificationError::NotFound(id) => {
            Status::not_found(format!("Notification not found: {}", id))
        }
        NotificationError::AlreadySent => {
            Status::failed_precondition("Notification already sent")
        }
        NotificationError::TemplateMissing(t) => {
            Status::invalid_argument(format!("Template not found: {}", t))
        }
        NotificationError::EmailProviderUnavailable => {
            Status::unavailable("Email provider unavailable")
        }
        NotificationError::SmsProviderUnavailable => {
            Status::unavailable("SMS provider unavailable")
        }
        NotificationError::MaxRetriesExceeded => {
            Status::failed_precondition("Max retries exceeded")
        }
    }
}

impl From<NotificationError> for Status {
    fn from(e: NotificationError) -> Self {
        notification_error_to_status(e)
    }
}

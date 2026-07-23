//! gRPC service implementation for notification-service (BC-14).
//! Translates between protobuf types and domain types for notification delivery.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{DeliveryStatus, NotificationChannel, NotificationError, Webhook};
use crate::queries::QueryHandler;
use crate::repository::WebhookRepository;
use platform_proto::common::Timestamp;
use platform_proto::notification::notification_service_server::NotificationService;
use platform_proto::notification::*;

pub struct NotificationGrpcService<C, Q, R> {
    commands: C,
    queries: Q,
    webhook_repo: R,
}

impl<C, Q, R> NotificationGrpcService<C, Q, R> {
    pub fn new(commands: C, queries: Q, webhook_repo: R) -> Self {
        Self { commands, queries, webhook_repo }
    }
}

#[tonic::async_trait]
impl<C, Q, R> NotificationService for NotificationGrpcService<C, Q, R>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
    R: WebhookRepository + Send + Sync + 'static,
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
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        if req.url.is_empty() {
            return Err(Status::invalid_argument("url is required"));
        }

        let event_types: Vec<String> = if req.event_types.is_empty() {
            vec!["*".to_string()] // Subscribe to all events by default
        } else {
            req.event_types.into_iter().filter(|e| !e.is_empty()).collect()
        };

        let (webhook, raw_secret) = Webhook::new(operator_id, req.url, event_types);

        match self.webhook_repo.save(&webhook).await {
            Ok(()) => {
                Ok(Response::new(RegisterWebhookResponse {
                    webhook_id: webhook.webhook_id.to_string(),
                    secret: raw_secret,
                    status: "active".into(),
                }))
            }
            Err(e) => Err(notification_error_to_status(e)),
        }
    }

    async fn list_webhooks(
        &self,
        request: Request<ListWebhooksRequest>,
    ) -> Result<Response<ListWebhooksResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        match self.webhook_repo.find_by_operator(operator_id).await {
            Ok(webhooks) => {
                let proto_webhooks: Vec<WebhookView> = webhooks
                    .into_iter()
                    .map(|w| WebhookView {
                        webhook_id: w.webhook_id.to_string(),
                        url: w.url,
                        event_types: w.events,
                        status: if w.is_active { "active".into() } else { "inactive".into() },
                        created_at: Some(Timestamp {
                            unix_ms: w.created_at.timestamp_millis(),
                        }),
                    })
                    .collect();

                Ok(Response::new(ListWebhooksResponse {
                    webhooks: proto_webhooks,
                }))
            }
            Err(e) => Err(notification_error_to_status(e)),
        }
    }

    async fn delete_webhook(
        &self,
        request: Request<DeleteWebhookRequest>,
    ) -> Result<Response<DeleteWebhookResponse>, Status> {
        let req = request.into_inner();
        let webhook_id = parse_uuid(&req.webhook_id, "webhook_id")?;

        match self.webhook_repo.load(webhook_id).await {
            Ok(Some(_webhook)) => {
                match self.webhook_repo.delete(webhook_id).await {
                    Ok(()) => Ok(Response::new(DeleteWebhookResponse { deleted: true })),
                    Err(e) => Err(notification_error_to_status(e)),
                }
            }
            Ok(None) => Err(Status::not_found("Webhook not found")),
            Err(e) => Err(notification_error_to_status(e)),
        }
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
        NotificationError::WebhookNotFound(id) => {
            Status::not_found(format!("Webhook not found: {}", id))
        }
    }
}

impl From<NotificationError> for Status {
    fn from(e: NotificationError) -> Self {
        notification_error_to_status(e)
    }
}

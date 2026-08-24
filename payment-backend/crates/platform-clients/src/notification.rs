//! Notification Service client (BC-14).
//! Email/SMS delivery, webhook management.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::notification::notification_service_client::NotificationServiceClient;
use platform_proto::notification::{
    SendEmailRequest, SendEmailResponse,
    SendSmsRequest, SendSmsResponse,
    GetDeliveryStatusRequest, GetDeliveryStatusResponse,
    RegisterWebhookRequest, RegisterWebhookResponse,
    ListWebhooksRequest, ListWebhooksResponse,
    DeleteWebhookRequest, DeleteWebhookResponse,
};

#[derive(Debug, Clone)]
pub struct NotificationClient {
    conn: ServiceConnection,
}

impl NotificationClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("notification-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("notification-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("notification-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> NotificationServiceClient<tonic::transport::Channel> {
        NotificationServiceClient::new(self.conn.channel().clone())
    }

    pub async fn send_email(&self, req: SendEmailRequest) -> Result<SendEmailResponse, ClientError> {
        self.client().await.send_email(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }

    pub async fn send_sms(&self, req: SendSmsRequest) -> Result<SendSmsResponse, ClientError> {
        self.client().await.send_sms(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }

    pub async fn get_delivery_status(&self, req: GetDeliveryStatusRequest) -> Result<GetDeliveryStatusResponse, ClientError> {
        self.client().await.get_delivery_status(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }

    pub async fn register_webhook(&self, req: RegisterWebhookRequest) -> Result<RegisterWebhookResponse, ClientError> {
        self.client().await.register_webhook(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }

    pub async fn list_webhooks(&self, req: ListWebhooksRequest) -> Result<ListWebhooksResponse, ClientError> {
        self.client().await.list_webhooks(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }

    pub async fn delete_webhook(&self, req: DeleteWebhookRequest) -> Result<DeleteWebhookResponse, ClientError> {
        self.client().await.delete_webhook(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "notification-service".into(), source: e })
    }
}

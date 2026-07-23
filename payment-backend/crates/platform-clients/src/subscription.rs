//! Subscription Billing Service client (BC-08).
//! Subscription lifecycle: create, cancel, pause, resume.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::subscription::subscription_service_client::SubscriptionServiceClient;
use platform_proto::subscription::{
    CreateSubscriptionRequest, CreateSubscriptionResponse,
    GetSubscriptionRequest, SubscriptionView,
    CancelSubscriptionRequest, CancelSubscriptionResponse,
    PauseSubscriptionRequest, PauseSubscriptionResponse,
    ResumeSubscriptionRequest, ResumeSubscriptionResponse,
    ListSubscriptionsRequest, ListSubscriptionsResponse,
};

#[derive(Debug, Clone)]
pub struct SubscriptionClient {
    conn: ServiceConnection,
}

impl SubscriptionClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("subscription-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("subscription-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("subscription-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> SubscriptionServiceClient<tonic::transport::Channel> {
        SubscriptionServiceClient::new(self.conn.channel().clone())
    }

    pub async fn create_subscription(&self, req: CreateSubscriptionRequest) -> Result<CreateSubscriptionResponse, ClientError> {
        self.client().await.create_subscription(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }

    pub async fn get_subscription(&self, req: GetSubscriptionRequest) -> Result<SubscriptionView, ClientError> {
        self.client().await.get_subscription(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }

    pub async fn cancel_subscription(&self, req: CancelSubscriptionRequest) -> Result<CancelSubscriptionResponse, ClientError> {
        self.client().await.cancel_subscription(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }

    pub async fn pause_subscription(&self, req: PauseSubscriptionRequest) -> Result<PauseSubscriptionResponse, ClientError> {
        self.client().await.pause_subscription(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }

    pub async fn resume_subscription(&self, req: ResumeSubscriptionRequest) -> Result<ResumeSubscriptionResponse, ClientError> {
        self.client().await.resume_subscription(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }

    pub async fn list_subscriptions(&self, req: ListSubscriptionsRequest) -> Result<ListSubscriptionsResponse, ClientError> {
        self.client().await.list_subscriptions(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "subscription-service".into(), source: e })
    }
}

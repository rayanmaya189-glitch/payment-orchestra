//! Payment Link Service client (BC-07).
//! Hosted payment link management.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::payment_link::payment_link_service_client::PaymentLinkServiceClient;
use platform_proto::payment_link::{
    CreatePaymentLinkRequest, CreatePaymentLinkResponse,
    GetPaymentLinkRequest, PaymentLinkView,
    ListPaymentLinksRequest, ListPaymentLinksResponse,
    ExpirePaymentLinkRequest, ExpirePaymentLinkResponse,
};

#[derive(Debug, Clone)]
pub struct PaymentLinkClient {
    conn: ServiceConnection,
}

impl PaymentLinkClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("payment-link-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("payment-link-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("payment-link-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> PaymentLinkServiceClient<tonic::transport::Channel> {
        PaymentLinkServiceClient::new(self.conn.channel().clone())
    }

    pub async fn create_payment_link(&self, req: CreatePaymentLinkRequest) -> Result<CreatePaymentLinkResponse, ClientError> {
        self.client().await.create_payment_link(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "payment-link-service".into(), source: e })
    }

    pub async fn get_payment_link(&self, req: GetPaymentLinkRequest) -> Result<PaymentLinkView, ClientError> {
        self.client().await.get_payment_link(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "payment-link-service".into(), source: e })
    }

    pub async fn list_payment_links(&self, req: ListPaymentLinksRequest) -> Result<ListPaymentLinksResponse, ClientError> {
        self.client().await.list_payment_links(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "payment-link-service".into(), source: e })
    }

    pub async fn expire_payment_link(&self, req: ExpirePaymentLinkRequest) -> Result<ExpirePaymentLinkResponse, ClientError> {
        self.client().await.expire_payment_link(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "payment-link-service".into(), source: e })
    }
}

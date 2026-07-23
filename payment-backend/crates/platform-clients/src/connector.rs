//! Connector Gateway gRPC client — merchant-acquirer link management.
//!
//! Wraps the MerchantAcquirerLinkService (defined in connector.proto)
//! for managing operator↔acquirer connections.

use platform_proto::connector::merchant_acquirer_link_service_client::MerchantAcquirerLinkServiceClient;
use platform_proto::connector::{
    CreateLinkRequest, CreateLinkResponse,
    TestConnectionRequest, TestConnectionResponse,
    RotateCredentialsRequest, RotateCredentialsResponse,
    DisableLinkRequest, DisableLinkResponse,
    EnableLinkRequest, EnableLinkResponse,
    UpdateLinkMetadataRequest, UpdateLinkMetadataResponse,
    GetLinkRequest, GetLinkResponse,
    ListLinksRequest, ListLinksResponse,
};

use crate::client::{ClientError, ServiceConnection};

/// Client for the Merchant Acquirer Link service (connector gateway).
#[derive(Debug, Clone)]
pub struct ConnectorClient {
    conn: ServiceConnection,
}

impl ConnectorClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("connector-gateway", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("connector-gateway", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("connector-gateway", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> MerchantAcquirerLinkServiceClient<tonic::transport::Channel> {
        MerchantAcquirerLinkServiceClient::new(self.conn.channel().clone())
    }

    pub async fn create_link(&self, req: CreateLinkRequest) -> Result<CreateLinkResponse, ClientError> {
        self.client().await.create_link(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn test_connection(&self, req: TestConnectionRequest) -> Result<TestConnectionResponse, ClientError> {
        self.client().await.test_connection(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn rotate_credentials(&self, req: RotateCredentialsRequest) -> Result<RotateCredentialsResponse, ClientError> {
        self.client().await.rotate_credentials(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn disable_link(&self, req: DisableLinkRequest) -> Result<DisableLinkResponse, ClientError> {
        self.client().await.disable_link(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn enable_link(&self, req: EnableLinkRequest) -> Result<EnableLinkResponse, ClientError> {
        self.client().await.enable_link(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn update_link_metadata(&self, req: UpdateLinkMetadataRequest) -> Result<UpdateLinkMetadataResponse, ClientError> {
        self.client().await.update_link_metadata(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn get_link(&self, req: GetLinkRequest) -> Result<GetLinkResponse, ClientError> {
        self.client().await.get_link(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }

    pub async fn list_links(&self, req: ListLinksRequest) -> Result<ListLinksResponse, ClientError> {
        self.client().await.list_links(tonic::Request::new(req)).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "connector-gateway".into(), source: e })
    }
}

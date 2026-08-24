//! Merchant Acquirer Link gRPC client — gateway profile and link queries.
//!
//! Used by Orchestration Service to discover active acquirer links
//! and check connection health for routing decisions.

use platform_proto::connector::merchant_acquirer_link_service_client::MerchantAcquirerLinkServiceClient;
use platform_proto::connector::{
    GetLinkRequest, GetLinkResponse,
    ListLinksRequest, ListLinksResponse,
    TestConnectionRequest, TestConnectionResponse,
};

use crate::client::{ClientError, ServiceConnection};

/// Client for the Merchant Acquirer Link service (BYOK Core).
#[derive(Debug, Clone)]
pub struct MerchantLinkClient {
    conn: ServiceConnection,
}

impl MerchantLinkClient {
    /// Create a new Merchant Link client connecting to the given address.
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("merchant-acquirer-link-service", addr).await?;
        Ok(Self { conn })
    }

    /// Create a lazy Merchant Link client (connects on first RPC).
    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("merchant-acquirer-link-service", addr)?;
        Ok(Self { conn })
    }

    /// Get a specific acquirer link by ID.
    pub async fn get_link(
        &self,
        link_id: String,
    ) -> Result<GetLinkResponse, ClientError> {
        let mut client = MerchantAcquirerLinkServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(GetLinkRequest { link_id });
        let response = client
            .get_link(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "merchant-acquirer-link-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// List all links for an operator, with optional status filter.
    pub async fn list_links(
        &self,
        operator_id: String,
        status_filter: String,
    ) -> Result<ListLinksResponse, ClientError> {
        let mut client = MerchantAcquirerLinkServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(ListLinksRequest {
            operator_id,
            status_filter,
        });
        let response = client
            .list_links(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "merchant-acquirer-link-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Test connection for a specific link.
    pub async fn test_connection(
        &self,
        link_id: String,
    ) -> Result<TestConnectionResponse, ClientError> {
        let mut client = MerchantAcquirerLinkServiceClient::new(self.conn.channel().clone());
        let request = tonic::Request::new(TestConnectionRequest { link_id });
        let response = client
            .test_connection(request)
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "merchant-acquirer-link-service".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }
}

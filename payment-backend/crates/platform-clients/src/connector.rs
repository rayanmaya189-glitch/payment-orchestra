//! Connector Gateway gRPC client — payment routing operations.
//!
//! Used by Orchestration Service to authorize, capture, refund,
//! and void transactions through the selected acquirer.

use platform_proto::connector::connector_gateway_service_client::ConnectorGatewayServiceClient;
use platform_proto::connector::{
    AuthorizeRequest, AuthorizeResponse,
    CaptureRequest, CaptureResponse,
    RefundRequest, RefundResponse,
    VoidRequest, VoidResponse,
};

use crate::client::{ClientError, ServiceConnection};

/// Client for the Connector Gateway service (BC-04).
///
/// Handles the low-level payment operations with acquirers.
#[derive(Debug, Clone)]
pub struct ConnectorClient {
    conn: ServiceConnection,
}

impl ConnectorClient {
    /// Create a new Connector client connecting to the given address.
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("connector-gateway", addr).await?;
        Ok(Self { conn })
    }

    /// Create a lazy Connector client (connects on first RPC).
    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("connector-gateway", addr)?;
        Ok(Self { conn })
    }

    /// Authorize a payment through the selected acquirer.
    pub async fn authorize(
        &self,
        request: AuthorizeRequest,
    ) -> Result<AuthorizeResponse, ClientError> {
        let mut client = ConnectorGatewayServiceClient::new(self.conn.channel().clone());
        let response = client
            .authorize(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "connector-gateway".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Capture an authorized payment.
    pub async fn capture(
        &self,
        request: CaptureRequest,
    ) -> Result<CaptureResponse, ClientError> {
        let mut client = ConnectorGatewayServiceClient::new(self.conn.channel().clone());
        let response = client
            .capture(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "connector-gateway".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Refund a captured payment.
    pub async fn refund(
        &self,
        request: RefundRequest,
    ) -> Result<RefundResponse, ClientError> {
        let mut client = ConnectorGatewayServiceClient::new(self.conn.channel().clone());
        let response = client
            .refund(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "connector-gateway".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }

    /// Void an authorized (uncaptured) payment.
    pub async fn void(
        &self,
        request: VoidRequest,
    ) -> Result<VoidResponse, ClientError> {
        let mut client = ConnectorGatewayServiceClient::new(self.conn.channel().clone());
        let response = client
            .void(tonic::Request::new(request))
            .await
            .map_err(|e| ClientError::RpcFailed {
                service: "connector-gateway".into(),
                source: e,
            })?;
        Ok(response.into_inner())
    }
}

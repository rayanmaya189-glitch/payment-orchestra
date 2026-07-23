//! Operator Service client (BC-01).
//! Wraps the tonic-generated OperatorServiceClient for operator management.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::operator::operator_service_client::OperatorServiceClient;
use platform_proto::operator::{
    RegisterOperatorRequest, RegisterOperatorResponse,
    VerifyEmailRequest, VerifyEmailResponse,
    UpdateOperatorStatusRequest, UpdateOperatorStatusResponse,
    GetOperatorRequest, GetOperatorResponse,
    ListOperatorsRequest, ListOperatorsResponse,
};

/// Typed client for the Operator Service.
#[derive(Debug, Clone)]
pub struct OperatorClient {
    conn: ServiceConnection,
}

impl OperatorClient {
    /// Connect to the operator service at the given address.
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("operator-service", addr).await?;
        Ok(Self { conn })
    }

    /// Create a lazy connection to the operator service.
    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("operator-service", addr)?;
        Ok(Self { conn })
    }

    /// Resolve from etcd and connect.
    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("operator-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> OperatorServiceClient<tonic::transport::Channel> {
        OperatorServiceClient::new(self.conn.channel().clone())
    }

    pub async fn register_operator(&self, req: RegisterOperatorRequest) -> Result<RegisterOperatorResponse, ClientError> {
        self.client().await.register_operator(req).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "operator-service".into(), source: e })
    }

    pub async fn verify_email(&self, req: VerifyEmailRequest) -> Result<VerifyEmailResponse, ClientError> {
        self.client().await.verify_email(req).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "operator-service".into(), source: e })
    }

    pub async fn update_operator_status(&self, req: UpdateOperatorStatusRequest) -> Result<UpdateOperatorStatusResponse, ClientError> {
        self.client().await.update_operator_status(req).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "operator-service".into(), source: e })
    }

    pub async fn get_operator(&self, req: GetOperatorRequest) -> Result<GetOperatorResponse, ClientError> {
        self.client().await.get_operator(req).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "operator-service".into(), source: e })
    }

    pub async fn list_operators(&self, req: ListOperatorsRequest) -> Result<ListOperatorsResponse, ClientError> {
        self.client().await.list_operators(req).await
            .map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "operator-service".into(), source: e })
    }
}

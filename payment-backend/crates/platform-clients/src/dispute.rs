//! Dispute Service client (BC-10).
//! Chargeback lifecycle: record, representment, resolve.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::dispute::dispute_service_client::DisputeServiceClient;
use platform_proto::dispute::{
    RecordChargebackRequest, RecordChargebackResponse,
    SubmitRepresentmentRequest, SubmitRepresentmentResponse,
    ResolveChargebackRequest, ResolveChargebackResponse,
    GetChargebackCaseRequest, ChargebackCaseView,
    ListChargebacksRequest, ListChargebacksResponse,
};

#[derive(Debug, Clone)]
pub struct DisputeClient {
    conn: ServiceConnection,
}

impl DisputeClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("dispute-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("dispute-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("dispute-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> DisputeServiceClient<tonic::transport::Channel> {
        DisputeServiceClient::new(self.conn.channel().clone())
    }

    pub async fn record_chargeback(&self, req: RecordChargebackRequest) -> Result<RecordChargebackResponse, ClientError> {
        self.client().await.record_chargeback(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "dispute-service".into(), source: e })
    }

    pub async fn submit_representment(&self, req: SubmitRepresentmentRequest) -> Result<SubmitRepresentmentResponse, ClientError> {
        self.client().await.submit_representment(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "dispute-service".into(), source: e })
    }

    pub async fn resolve_chargeback(&self, req: ResolveChargebackRequest) -> Result<ResolveChargebackResponse, ClientError> {
        self.client().await.resolve_chargeback(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "dispute-service".into(), source: e })
    }

    pub async fn get_chargeback_case(&self, req: GetChargebackCaseRequest) -> Result<ChargebackCaseView, ClientError> {
        self.client().await.get_chargeback_case(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "dispute-service".into(), source: e })
    }

    pub async fn list_chargebacks(&self, req: ListChargebacksRequest) -> Result<ListChargebacksResponse, ClientError> {
        self.client().await.list_chargebacks(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "dispute-service".into(), source: e })
    }
}

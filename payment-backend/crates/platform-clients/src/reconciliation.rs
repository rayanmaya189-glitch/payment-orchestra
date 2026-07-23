//! Reconciliation Service client (BC-09).
//! Settlement file ingestion, exception resolution, batch tracking.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::reconciliation::reconciliation_service_client::ReconciliationServiceClient;
use platform_proto::reconciliation::{
    IngestSettlementFileRequest, IngestSettlementFileResponse,
    GetExceptionsRequest, GetExceptionsResponse,
    ResolveExceptionRequest, ResolveExceptionResponse,
    GetBatchesRequest, GetBatchesResponse,
    GetReconciliationStatsRequest, GetReconciliationStatsResponse,
};

#[derive(Debug, Clone)]
pub struct ReconciliationClient {
    conn: ServiceConnection,
}

impl ReconciliationClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("reconciliation-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("reconciliation-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("reconciliation-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> ReconciliationServiceClient<tonic::transport::Channel> {
        ReconciliationServiceClient::new(self.conn.channel().clone())
    }

    pub async fn ingest_settlement_file(&self, req: IngestSettlementFileRequest) -> Result<IngestSettlementFileResponse, ClientError> {
        self.client().await.ingest_settlement_file(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "reconciliation-service".into(), source: e })
    }

    pub async fn get_exceptions(&self, req: GetExceptionsRequest) -> Result<GetExceptionsResponse, ClientError> {
        self.client().await.get_reconciliation_exceptions(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "reconciliation-service".into(), source: e })
    }

    pub async fn resolve_exception(&self, req: ResolveExceptionRequest) -> Result<ResolveExceptionResponse, ClientError> {
        self.client().await.resolve_exception(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "reconciliation-service".into(), source: e })
    }

    pub async fn get_batches(&self, req: GetBatchesRequest) -> Result<GetBatchesResponse, ClientError> {
        self.client().await.get_settlement_batches(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "reconciliation-service".into(), source: e })
    }

    pub async fn get_reconciliation_stats(&self, req: GetReconciliationStatsRequest) -> Result<GetReconciliationStatsResponse, ClientError> {
        self.client().await.get_reconciliation_stats(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "reconciliation-service".into(), source: e })
    }
}

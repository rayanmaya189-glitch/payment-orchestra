//! Saga Coordinator client (BC-17).
//! Distributed saga instance tracking, retry, and compensation.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::saga::saga_service_client::SagaServiceClient;
use platform_proto::saga::{
    GetSagaInstanceRequest, SagaInstanceView,
    ListSagaInstancesRequest, ListSagaInstancesResponse,
    RetrySagaStepRequest, RetrySagaStepResponse,
    CompensateSagaRequest, CompensateSagaResponse,
};

#[derive(Debug, Clone)]
pub struct SagaCoordinatorClient {
    conn: ServiceConnection,
}

impl SagaCoordinatorClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("saga-coordinator", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("saga-coordinator", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("saga-coordinator", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> SagaServiceClient<tonic::transport::Channel> {
        SagaServiceClient::new(self.conn.channel().clone())
    }

    pub async fn get_saga_instance(&self, req: GetSagaInstanceRequest) -> Result<SagaInstanceView, ClientError> {
        self.client().await.get_saga_instance(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "saga-coordinator".into(), source: e })
    }

    pub async fn list_saga_instances(&self, req: ListSagaInstancesRequest) -> Result<ListSagaInstancesResponse, ClientError> {
        self.client().await.list_saga_instances(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "saga-coordinator".into(), source: e })
    }

    pub async fn retry_saga_step(&self, req: RetrySagaStepRequest) -> Result<RetrySagaStepResponse, ClientError> {
        self.client().await.retry_saga_step(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "saga-coordinator".into(), source: e })
    }

    pub async fn compensate_saga(&self, req: CompensateSagaRequest) -> Result<CompensateSagaResponse, ClientError> {
        self.client().await.compensate_saga(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "saga-coordinator".into(), source: e })
    }
}

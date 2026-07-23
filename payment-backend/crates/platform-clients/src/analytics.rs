//! Analytics Service client (BC-15).
//! Metrics, reporting, analytics queries.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::analytics::analytics_service_client::AnalyticsServiceClient;
use platform_proto::analytics::{
    GetAuthRatesRequest, GetAuthRatesResponse,
    GetDeclineReasonsRequest, GetDeclineReasonsResponse,
    GetSettlementStatusRequest, GetSettlementStatusResponse,
    GetFeeAnalysisRequest, GetFeeAnalysisResponse,
    GetChargebackTrendsRequest, GetChargebackTrendsResponse,
    GetVolumeOverTimeRequest, GetVolumeOverTimeResponse,
};

#[derive(Debug, Clone)]
pub struct AnalyticsClient {
    conn: ServiceConnection,
}

impl AnalyticsClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("analytics-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("analytics-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("analytics-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> AnalyticsServiceClient<tonic::transport::Channel> {
        AnalyticsServiceClient::new(self.conn.channel().clone())
    }

    pub async fn get_auth_rates(&self, req: GetAuthRatesRequest) -> Result<GetAuthRatesResponse, ClientError> {
        self.client().await.get_authorization_rates(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }

    pub async fn get_decline_reasons(&self, req: GetDeclineReasonsRequest) -> Result<GetDeclineReasonsResponse, ClientError> {
        self.client().await.get_decline_reasons(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }

    pub async fn get_settlement_status(&self, req: GetSettlementStatusRequest) -> Result<GetSettlementStatusResponse, ClientError> {
        self.client().await.get_settlement_status(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }

    pub async fn get_fee_analysis(&self, req: GetFeeAnalysisRequest) -> Result<GetFeeAnalysisResponse, ClientError> {
        self.client().await.get_fee_analysis(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }

    pub async fn get_chargeback_trends(&self, req: GetChargebackTrendsRequest) -> Result<GetChargebackTrendsResponse, ClientError> {
        self.client().await.get_chargeback_trends(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }

    pub async fn get_volume_over_time(&self, req: GetVolumeOverTimeRequest) -> Result<GetVolumeOverTimeResponse, ClientError> {
        self.client().await.get_volume_over_time(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "analytics-service".into(), source: e })
    }
}

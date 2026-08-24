//! Compliance Service client (BC-03).
//! KYB case management, AML monitoring, and SAR reporting.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::compliance::compliance_service_client::ComplianceServiceClient;
use platform_proto::compliance::{
    SubmitKybEvidenceRequest, SubmitKybEvidenceResponse,
    ReviewKybCaseRequest, ReviewKybCaseResponse,
    GetKybCaseRequest, GetKybCaseResponse,
    ListPendingKybCasesRequest, ListPendingKybCasesResponse,
    ScanTransactionRequest, ScanTransactionResponse,
    ListAmlAlertsRequest, ListAmlAlertsResponse,
    ReviewAmlAlertRequest, ReviewAmlAlertResponse,
};

/// Typed client for the Compliance Service.
#[derive(Debug, Clone)]
pub struct ComplianceClient {
    conn: ServiceConnection,
}

impl ComplianceClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("compliance-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("compliance-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("compliance-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> ComplianceServiceClient<tonic::transport::Channel> {
        ComplianceServiceClient::new(self.conn.channel().clone())
    }

    pub async fn submit_kyb_evidence(&self, req: SubmitKybEvidenceRequest) -> Result<SubmitKybEvidenceResponse, ClientError> {
        self.client().await.submit_kyb_evidence(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn review_kyb_case(&self, req: ReviewKybCaseRequest) -> Result<ReviewKybCaseResponse, ClientError> {
        self.client().await.review_kyb_case(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn get_kyb_case(&self, req: GetKybCaseRequest) -> Result<GetKybCaseResponse, ClientError> {
        self.client().await.get_kyb_case(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn list_pending_kyb_cases(&self, req: ListPendingKybCasesRequest) -> Result<ListPendingKybCasesResponse, ClientError> {
        self.client().await.list_pending_kyb_cases(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn scan_transaction(&self, req: ScanTransactionRequest) -> Result<ScanTransactionResponse, ClientError> {
        self.client().await.scan_transaction(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn list_aml_alerts(&self, req: ListAmlAlertsRequest) -> Result<ListAmlAlertsResponse, ClientError> {
        self.client().await.list_aml_alerts(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }

    pub async fn review_aml_alert(&self, req: ReviewAmlAlertRequest) -> Result<ReviewAmlAlertResponse, ClientError> {
        self.client().await.review_aml_alert(req).await
            .map(|r| r.into_inner()).map_err(|e| ClientError::RpcFailed { service: "compliance-service".into(), source: e })
    }
}

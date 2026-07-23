//! Invoice Service client (BC-06).
//! Invoice lifecycle: create, send, cancel, payment tracking.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::invoice::invoice_service_client::InvoiceServiceClient;
use platform_proto::invoice::{
    CreateInvoiceRequest, CreateInvoiceResponse,
    GetInvoiceRequest, InvoiceView,
    ListInvoicesRequest, ListInvoicesResponse,
    SendInvoiceRequest, SendInvoiceResponse,
    CancelInvoiceRequest, CancelInvoiceResponse,
};

#[derive(Debug, Clone)]
pub struct InvoiceClient {
    conn: ServiceConnection,
}

impl InvoiceClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("invoice-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("invoice-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("invoice-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> InvoiceServiceClient<tonic::transport::Channel> {
        InvoiceServiceClient::new(self.conn.channel().clone())
    }

    pub async fn create_invoice(&self, req: CreateInvoiceRequest) -> Result<CreateInvoiceResponse, ClientError> {
        self.client().await.create_invoice(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "invoice-service".into(), source: e })
    }

    pub async fn get_invoice(&self, req: GetInvoiceRequest) -> Result<InvoiceView, ClientError> {
        self.client().await.get_invoice(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "invoice-service".into(), source: e })
    }

    pub async fn list_invoices(&self, req: ListInvoicesRequest) -> Result<ListInvoicesResponse, ClientError> {
        self.client().await.list_invoices(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "invoice-service".into(), source: e })
    }

    pub async fn send_invoice(&self, req: SendInvoiceRequest) -> Result<SendInvoiceResponse, ClientError> {
        self.client().await.send_invoice(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "invoice-service".into(), source: e })
    }

    pub async fn cancel_invoice(&self, req: CancelInvoiceRequest) -> Result<CancelInvoiceResponse, ClientError> {
        self.client().await.cancel_invoice(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "invoice-service".into(), source: e })
    }
}

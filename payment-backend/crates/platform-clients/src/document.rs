//! Document Service client (BC-13).
//! Document upload, OCR pipeline, secure retrieval.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::document::document_service_client::DocumentServiceClient;
use platform_proto::document::{
    UploadDocumentRequest, UploadDocumentResponse,
    GetDocumentUrlRequest, GetDocumentUrlResponse,
    ListDocumentsRequest, ListDocumentsResponse,
    DeleteDocumentRequest, DeleteDocumentResponse,
};

#[derive(Debug, Clone)]
pub struct DocumentClient {
    conn: ServiceConnection,
}

impl DocumentClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("document-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("document-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("document-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> DocumentServiceClient<tonic::transport::Channel> {
        DocumentServiceClient::new(self.conn.channel().clone())
    }

    pub async fn upload_document(&self, req: UploadDocumentRequest) -> Result<UploadDocumentResponse, ClientError> {
        self.client().await.upload_document(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "document-service".into(), source: e })
    }

    pub async fn get_document_url(&self, req: GetDocumentUrlRequest) -> Result<GetDocumentUrlResponse, ClientError> {
        self.client().await.get_document_url(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "document-service".into(), source: e })
    }

    pub async fn list_documents(&self, req: ListDocumentsRequest) -> Result<ListDocumentsResponse, ClientError> {
        self.client().await.list_documents(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "document-service".into(), source: e })
    }

    pub async fn delete_document(&self, req: DeleteDocumentRequest) -> Result<DeleteDocumentResponse, ClientError> {
        self.client().await.delete_document(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "document-service".into(), source: e })
    }
}

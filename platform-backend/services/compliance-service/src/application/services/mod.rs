use async_trait::async_trait;

use crate::application::commands::*;
use crate::application::queries::*;
use platform_error::PlatformError;

#[async_trait]
pub trait ComplianceService: Send + Sync {
    async fn create_kyb_case(&self, cmd: CreateKybCaseCommand) -> Result<KybCaseResponse, PlatformError>;
    async fn upload_document(&self, cmd: UploadDocumentCommand) -> Result<(), PlatformError>;
    async fn verify_document(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError>;
    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError>;
    async fn decide_case(&self, cmd: DecideKybCaseCommand) -> Result<(), PlatformError>;
    async fn request_documents(&self, cmd: RequestDocumentsCommand) -> Result<(), PlatformError>;
    async fn get_case(&self, query: GetKybCaseQuery) -> Result<KybCaseResponse, PlatformError>;
    async fn list_cases(&self, query: ListKybCasesQuery) -> Result<Vec<KybCaseResponse>, PlatformError>;
}

#[derive(Debug, Clone)]
pub struct KybCaseResponse {
    pub id: uuid::Uuid,
    pub operator_id: uuid::Uuid,
    pub status: String,
    pub assigned_officer: Option<uuid::Uuid>,
    pub document_count: usize,
    pub verified_count: usize,
    pub risk_score: Option<f64>,
    pub submitted_at: String,
    pub decided_at: Option<String>,
}

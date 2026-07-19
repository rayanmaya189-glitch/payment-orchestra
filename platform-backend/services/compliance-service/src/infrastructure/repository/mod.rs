use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::KybCase;
use platform_error::PlatformError;

#[async_trait]
pub trait KybCaseRepository: Send + Sync {
    async fn save(&self, case: &KybCase) -> Result<(), PlatformError>;
    async fn load(&self, id: Uuid) -> Result<Option<KybCase>, PlatformError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, PlatformError>;
    async fn list(&self, status: Option<&str>, limit: u64, offset: u64) -> Result<Vec<KybCase>, PlatformError>;
    async fn save_document(&self, document: &crate::domain::aggregates::KybDocument) -> Result<(), PlatformError>;
    async fn load_documents(&self, kyb_case_id: Uuid) -> Result<Vec<crate::domain::aggregates::KybDocument>, PlatformError>;
    async fn update_document_verification(&self, document_id: Uuid, verified: bool) -> Result<(), PlatformError>;
}

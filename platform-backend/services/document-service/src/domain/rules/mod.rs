use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Document;
use platform_error::PlatformError;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Document>, PlatformError>;
    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Document>, PlatformError>;
    async fn save(&self, document: &Document) -> Result<(), PlatformError>;
    async fn count_by_operator_and_type(
        &self,
        operator_id: Uuid,
        document_type: &str,
    ) -> Result<i64, PlatformError>;
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, PlatformError>;
    async fn download(&self, key: &str) -> Result<Vec<u8>, PlatformError>;
    async fn delete(&self, key: &str) -> Result<(), PlatformError>;
    async fn exists(&self, key: &str) -> Result<bool, PlatformError>;
}

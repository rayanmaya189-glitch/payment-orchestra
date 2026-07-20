use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Document;
use platform_error::PlatformError;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Document>, PlatformError>;
    async fn save(&self, document: &Document) -> Result<(), PlatformError>;
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<(), PlatformError>;
    async fn download(&self, key: &str) -> Result<Vec<u8>, PlatformError>;
}

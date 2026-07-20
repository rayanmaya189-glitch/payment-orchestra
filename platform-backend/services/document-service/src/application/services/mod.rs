use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::Document;
use crate::domain::rules::DocumentRepository;
use platform_error::PlatformError;
use sha2::{Sha256, Digest};

pub struct DocumentServiceImpl { repo: Box<dyn DocumentRepository>, db: DatabaseConnection }
impl DocumentServiceImpl { pub fn new(repo: Box<dyn DocumentRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait DocumentService: Send + Sync {
    async fn upload(&self, cmd: UploadDocumentCommand) -> Result<Uuid, PlatformError>;
    async fn verify(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<Document, PlatformError>;
}

#[async_trait]
impl DocumentService for DocumentServiceImpl {
    async fn upload(&self, cmd: UploadDocumentCommand) -> Result<Uuid, PlatformError> {
        let mut hasher = Sha256::new();
        hasher.update(&cmd.file_data);
        let hash = hex::encode(hasher.finalize());
        let storage_key = format!("{}/{}/{}", cmd.operator_id, cmd.document_type, uuid::Uuid::now_v7());
        let mut doc = Document::new(cmd.operator_id, cmd.document_type, cmd.filename, cmd.content_type, cmd.file_size, storage_key, hash, cmd.uploaded_by);
        self.repo.save(&doc).await?;
        Ok(doc.document_id)
    }

    async fn verify(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError> {
        let mut doc = self.repo.find_by_id(cmd.document_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "document".into(), id: cmd.document_id })?;
        if cmd.approved { doc.verify(&cmd.notes); } else { doc.reject(&cmd.notes); }
        self.repo.save(&doc).await
    }

    async fn get(&self, id: Uuid) -> Result<Document, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "document".into(), id })
    }
}

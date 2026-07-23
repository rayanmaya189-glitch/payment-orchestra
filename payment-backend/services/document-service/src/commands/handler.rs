//! Document Management command handlers — BC-13

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn upload_document(&self, cmd: UploadDocumentCommand) -> Result<DocumentRecord, DocumentError>;
    async fn start_ocr(&self, cmd: StartOcrCommand) -> Result<DocumentRecord, DocumentError>;
    async fn complete_ocr(&self, cmd: CompleteOcrCommand) -> Result<DocumentRecord, DocumentError>;
    async fn fail_ocr(&self, cmd: FailOcrCommand) -> Result<DocumentRecord, DocumentError>;
    async fn delete_document(&self, cmd: DeleteDocumentCommand) -> Result<(), DocumentError>;
}

pub struct DocumentCommandHandler<R: DocumentRepository> {
    repo: R,
}

impl<R: DocumentRepository> DocumentCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: DocumentRepository + Send + Sync> CommandHandler for DocumentCommandHandler<R> {
    async fn upload_document(&self, cmd: UploadDocumentCommand) -> Result<DocumentRecord, DocumentError> {
        let storage_key = format!(
            "{}/{}_{}",
            cmd.operator_id,
            Uuid::now_v7(),
            sanitize_filename(&cmd.filename)
        );

        let record = DocumentRecord::new(
            cmd.operator_id,
            cmd.uploaded_by,
            cmd.category,
            cmd.filename,
            cmd.content_type,
            cmd.body.len() as i64,
            storage_key,
        )?;

        self.repo.save(&record).await?;
        Ok(record)
    }

    async fn start_ocr(&self, cmd: StartOcrCommand) -> Result<DocumentRecord, DocumentError> {
        let mut record = self.repo.load(cmd.document_id).await?
            .ok_or(DocumentError::NotFound(cmd.document_id))?;
        record.start_ocr()?;
        self.repo.save(&record).await?;
        Ok(record)
    }

    async fn complete_ocr(&self, cmd: CompleteOcrCommand) -> Result<DocumentRecord, DocumentError> {
        let mut record = self.repo.load(cmd.document_id).await?
            .ok_or(DocumentError::NotFound(cmd.document_id))?;
        record.complete_ocr(cmd.ocr_result)?;
        self.repo.save(&record).await?;
        Ok(record)
    }

    async fn fail_ocr(&self, cmd: FailOcrCommand) -> Result<DocumentRecord, DocumentError> {
        let mut record = self.repo.load(cmd.document_id).await?
            .ok_or(DocumentError::NotFound(cmd.document_id))?;
        record.fail_ocr(&cmd.error)?;
        self.repo.save(&record).await?;
        Ok(record)
    }

    async fn delete_document(&self, cmd: DeleteDocumentCommand) -> Result<(), DocumentError> {
        self.repo.load(cmd.document_id).await?
            .ok_or(DocumentError::NotFound(cmd.document_id))?;
        self.repo.delete(cmd.document_id).await?;
        Ok(())
    }
}

/// Sanitize filename for storage: remove path separators, keep alphanumeric + dots + hyphens.
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

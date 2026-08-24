//! Document Management API surface — BC-13

pub mod grpc;

use crate::commands::*;
use crate::domain::{DocumentError, DocumentRecord};
use crate::queries::*;
use uuid::Uuid;

pub struct DocumentApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl DocumentApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    pub async fn upload_document(
        &self,
        cmd: UploadDocumentCommand,
    ) -> Result<DocumentRecord, DocumentError> {
        self.command_handler.upload_document(cmd).await
    }

    pub async fn start_ocr(&self, cmd: StartOcrCommand) -> Result<DocumentRecord, DocumentError> {
        self.command_handler.start_ocr(cmd).await
    }

    pub async fn complete_ocr(
        &self,
        cmd: CompleteOcrCommand,
    ) -> Result<DocumentRecord, DocumentError> {
        self.command_handler.complete_ocr(cmd).await
    }

    pub async fn fail_ocr(&self, cmd: FailOcrCommand) -> Result<DocumentRecord, DocumentError> {
        self.command_handler.fail_ocr(cmd).await
    }

    pub async fn delete_document(&self, cmd: DeleteDocumentCommand) -> Result<(), DocumentError> {
        self.command_handler.delete_document(cmd).await
    }

    pub async fn get_document(&self, id: Uuid) -> Result<DocumentRecord, DocumentError> {
        self.query_handler.get_document(id).await
    }

    pub async fn find_by_type(
        &self,
        operator_id: Uuid,
        category: &str,
    ) -> Result<Vec<DocumentRecord>, DocumentError> {
        self.query_handler.find_by_type(operator_id, category).await
    }

    pub async fn find_by_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<DocumentRecord>, DocumentError> {
        self.query_handler.find_by_operator(operator_id).await
    }
}
